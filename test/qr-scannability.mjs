// Can the QR code on screen actually be scanned?
//
// Every other test in this suite reads the pairing payload out of
// `BlipQR.lastRenderedText`, or decodes the canvas bitmap via
// `toDataURL()`. Both bypass the browser's own rendering: neither one
// ever looks at the pixels a player's phone would photograph. That gap
// hid a real defect. The code was drawn on a fixed ~400px grid and then
// given a CSS size the browser had to resample it to, and resampling a
// QR by a fractional factor merges and drops whole module rows. Swept
// across the plausible on-screen size range, 51-78% of sizes failed to
// decode -- including the 280px default the pairing modal shipped with.
// The failure is not gradual and gives no hint of itself: the code looks
// perfectly fine to a human.
//
// So this test screenshots the canvas element through the real
// compositor, at its real displayed size, and decodes those pixels.
//
// It also pins the other half, which matters just as much: when the
// screen is genuinely too small to show a scannable code, the player is
// told so, rather than being left pointing a camera at a code that
// cannot work.

import test from 'node:test';
import assert from 'node:assert/strict';
import { launchEngine } from './lib/engine.mjs';
import {
  createFileServer, HTTP_PORT, loadRally, openModal, clickHsBtn,
  makeSyntheticOfferSdp, QR_READY, evaluate, waitFor,
} from './lib/multiplayer-harness.mjs';

const ENGINE = process.env.BLIP_QR_ENGINE || 'chromium';

// Viewports a player plausibly holds this up on. The short ones are the
// point: `codeAreaSize()` derives the box from innerHeight, so a short
// window is exactly where the code gets squeezed.
const VIEWPORTS = [
  { name: 'desktop', width: 1280, height: 800 },
  { name: 'iPhone portrait', width: 390, height: 700 },
  { name: 'small phone portrait', width: 320, height: 568 },
  { name: 'phone landscape (short)', width: 700, height: 360 },
];

// Candidate counts spanning what a real offer/answer carries; more
// candidates means a longer SDP, which means more modules in the same box.
const PAYLOADS = [4, 8, 12];

/** Screenshot the live canvas element and decode those exact pixels. */
async function decodeOnScreen(page, cdp) {
  const el = await page.$('.blip-hs-panel canvas.blip-qr-canvas');
  assert.ok(el, 'no QR canvas on screen');
  const png = (await el.screenshot({ type: 'png' })).toString('base64');
  // Decoded back inside the page so jsQR sees the screenshot's own
  // pixels, at the size they were captured.
  return evaluate(cdp, `
    new Promise(function (resolve) {
      var img = new Image();
      img.onload = function () {
        var c = document.createElement('canvas');
        c.width = img.naturalWidth; c.height = img.naturalHeight;
        var x = c.getContext('2d');
        x.drawImage(img, 0, 0);
        var d = x.getImageData(0, 0, c.width, c.height);
        var got = jsQR(d.data, c.width, c.height);
        resolve({ w: c.width, h: c.height, text: got ? got.data : null });
      };
      img.onerror = function () { resolve({ w: 0, h: 0, text: null }); };
      img.src = 'data:image/png;base64,${png}';
    })`);
}

test(`the QR code on screen decodes (${ENGINE})`, async (t) => {
  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const { browser, page, cdp } = await launchEngine(ENGINE);
  t.after(async () => {
    await browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });

  for (const vp of VIEWPORTS) {
    for (const candidates of PAYLOADS) {
      await t.test(`${vp.name} ${vp.width}x${vp.height}, ${candidates} ICE candidates`, async () => {
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await loadRally(cdp);
        await openModal(cdp);
        await clickHsBtn(cdp, 'HOST');
        await waitFor(cdp, QR_READY, 20000);

        // Re-render the real on-screen canvas with a payload of known
        // size, through the same call and the same box the modal uses.
        const sdp = makeSyntheticOfferSdp(candidates);
        // Through the modal's own render path, so the warning behaviour
        // under test is the one players actually get.
        const res = await evaluate(cdp, `(function () {
          var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
          return window.__blipRenderCodeForTest(c, ${JSON.stringify(sdp)});
        })()`);
        const info = res.render;

        // The invariant the fix rests on: the bitmap is displayed at
        // exactly its own device-pixel size, so nothing is resampled.
        const dpr = await evaluate(cdp, 'window.devicePixelRatio || 1');
        assert.equal(info.bitmapPx, Math.round(info.cssPx * dpr),
          `bitmap ${info.bitmapPx}px displayed at ${info.cssPx} CSS px x${dpr} — fractional scaling`);
        assert.equal(info.bitmapPx % info.modules, 0,
          `bitmap ${info.bitmapPx}px is not a whole number of ${info.modules} modules`);

        const shot = await decodeOnScreen(page, cdp);

        if (info.scannable) {
          assert.equal(shot.text, sdp,
            `on-screen code did not decode at ${info.cssPxPerModule.toFixed(2)} CSS px/module ` +
            `(${info.modules} modules in ${info.cssPx}px, screenshot ${shot.w}x${shot.h})`);
        } else {
          // Too small to promise. The contract then is not "it decodes"
          // but "the player is told" — asserted below.
          t.diagnostic(`unscannable by design at ${vp.name}: ` +
            `${info.cssPxPerModule.toFixed(2)} CSS px/module, decoded=${shot.text === sdp}`);
          assert.ok(res.warnShown, 'a code too small to scan was shown with no warning');
          assert.match(res.warnText, /too small to scan/i);
        }
      });
    }
  }
});

test(`an unscannably small code always warns, and a normal one never does (${ENGINE})`, async (t) => {
  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const { browser, cdp } = await launchEngine(ENGINE);
  t.after(async () => {
    await browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });

  await loadRally(cdp);
  await openModal(cdp);
  await clickHsBtn(cdp, 'HOST');
  await waitFor(cdp, QR_READY, 20000);

  const renderAt = (fit, sdp) => evaluate(cdp, `(function () {
    var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
    return window.__blipRenderCodeForTest(c, ${JSON.stringify(sdp)}, ${fit});
  })()`);

  const big = makeSyntheticOfferSdp(12);

  await t.test('a cramped box warns', async () => {
    const res = await renderAt(120, big);
    assert.ok(!res.render.scannable, `120px box should not be scannable, got ${res.render.cssPxPerModule}`);
    assert.ok(res.warnShown, 'no visible warning for an unscannable code');
    assert.match(res.warnText, /too small to scan/i);
  });

  await t.test('the warning clears again once there is room', async () => {
    // Stale warnings are their own bug: a player who rotates the phone
    // and fixes the problem must not still be told it is broken.
    const res = await renderAt(400, big);
    assert.ok(res.render.scannable, `400px box should be scannable, got ${res.render.cssPxPerModule}`);
    assert.equal(res.warnShown, false, 'warning stayed up after the code became scannable');
  });
});

// The finding itself, kept as an executable demonstration rather than a
// paragraph in a commit message.
//
// The failure mode is worth understanding because it is so unlike a
// normal bug: the code renders perfectly, looks perfectly fine to a
// human, and decodes or does not depending on nothing the player can
// see or influence. It is not a gradual loss of quality with size -- it
// is a lottery held once per (payload size, display size) pair. A
// developer testing with one payload on one screen can easily see it
// work every time, ship it, and get "the camera doesn't work" reports
// from everyone whose SDP happened to be a different length.
test('fractional scaling is what breaks QR codes, and integer scaling is what fixes it', async (t) => {
  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const { browser, cdp } = await launchEngine(ENGINE);
  t.after(async () => {
    await browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  await loadRally(cdp);

  for (const candidates of [4, 8]) {
    const sdp = makeSyntheticOfferSdp(candidates);
    const result = await evaluate(cdp, `(function () {
      var text = ${JSON.stringify(sdp)};

      // The old behaviour: draw on a fixed ~400px grid (render() with no
      // box), then let the browser resample to whatever CSS size the
      // layout happened to produce.
      var legacy = document.createElement('canvas');
      window.BlipQR.render(legacy, text);

      function decodeAt(src, display) {
        var t = document.createElement('canvas');
        t.width = display; t.height = display;
        var x = t.getContext('2d');
        x.imageSmoothingEnabled = false; // matches image-rendering: pixelated
        x.drawImage(src, 0, 0, display, display);
        var d = x.getImageData(0, 0, display, display);
        var got = jsQR(d.data, display, display);
        return !!(got && got.data === text);
      }

      var tried = 0, failed = 0, failures = [];
      for (var s = 140; s <= 340; s++) {
        tried++;
        if (!decodeAt(legacy, s)) { failed++; if (failures.length < 6) failures.push(s); }
      }

      // The new behaviour: the bitmap is built to the box, one module
      // per whole number of device pixels, and displayed 1:1. Sweep the
      // same range of boxes and decode the bitmap that actually results.
      var fixedTried = 0, fixedFailed = 0, fixedFailures = [];
      for (var box = 140; box <= 340; box++) {
        var c = document.createElement('canvas');
        var info = window.BlipQR.render(c, text, { fitCssPx: box, dpr: 1 });
        fixedTried++;
        if (!decodeAt(c, c.width)) { fixedFailed++; if (fixedFailures.length < 6) fixedFailures.push(box); }
      }

      return {
        bytes: text.length, legacyBitmap: legacy.width,
        tried: tried, failed: failed, failures: failures,
        fixedTried: fixedTried, fixedFailed: fixedFailed, fixedFailures: fixedFailures
      };
    })()`);

    await t.test(`${result.bytes}-byte payload: the old fixed-grid + CSS-scale approach loses codes`, () => {
      t.diagnostic(`fractional scaling: ${result.failed}/${result.tried} display sizes failed ` +
        `(${Math.round(100 * result.failed / result.tried)}%), e.g. ${result.failures.join(', ')}px`);
      assert.ok(result.failed > 0,
        'expected fractional rescaling to destroy at least some codes; if this ever stops ' +
        'being true the fix below may no longer be load-bearing');
    });

    await t.test(`${result.bytes}-byte payload: sizing to whole modules decodes at every box size`, () => {
      assert.equal(result.fixedFailed, 0,
        `${result.fixedFailed}/${result.fixedTried} box sizes failed to decode ` +
        `(e.g. ${result.fixedFailures.join(', ')}px) — the module grid is not being respected`);
    });
  }
});

// A QR code is only as findable as its quiet zone. This is cheap to
// break accidentally (a tighter layout, a background colour change) and
// expensive to diagnose, because a code with no quiet zone frequently
// cannot be *located* at all rather than merely decoding poorly -- the
// camera sees something that looks right to a human and reports nothing.
test('the rendered code keeps a light quiet zone on all four sides', async (t) => {
  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const { browser, cdp } = await launchEngine(ENGINE);
  t.after(async () => {
    await browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  await loadRally(cdp);

  const edges = await evaluate(cdp, `(function () {
    var c = document.createElement('canvas');
    var info = window.BlipQR.render(c, ${JSON.stringify(makeSyntheticOfferSdp(4))}, { fitCssPx: 300, dpr: 1 });
    var x = c.getContext('2d');
    var d = x.getImageData(0, 0, c.width, c.height).data;
    function isLight(px, py) {
      var i = (py * c.width + px) * 4;
      return d[i] > 200 && d[i + 1] > 200 && d[i + 2] > 200;
    }
    // One module in from each edge, sampled along the whole side.
    var quiet = info.deviceCellPx * 4;
    var bad = { top: 0, bottom: 0, left: 0, right: 0 };
    for (var p = 0; p < c.width; p++) {
      for (var q = 0; q < quiet; q++) {
        if (!isLight(p, q)) bad.top++;
        if (!isLight(p, c.width - 1 - q)) bad.bottom++;
        if (!isLight(q, p)) bad.left++;
        if (!isLight(c.width - 1 - q, p)) bad.right++;
      }
    }
    return { quietPx: quiet, bad: bad, size: c.width };
  })()`);

  assert.ok(edges.quietPx > 0, 'no quiet zone was reserved at all');
  assert.deepEqual(edges.bad, { top: 0, bottom: 0, left: 0, right: 0 },
    `dark pixels found inside the ${edges.quietPx}px quiet zone`);
});
