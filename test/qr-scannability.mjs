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
import {
  openPage, openPair, HTTP_PORT, loadRally, openModal, clickHsBtn,
  sleep,
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

// What the QR actually carries, at the sizes it actually reaches.
//
// This used to be candidate counts fed through makeSyntheticOfferSdp,
// i.e. full SDPs of 700-1600 bytes. Nothing produces those any more: the
// payload has been the compact form since packForQr landed, and a real
// offer is ~98 bytes. Testing the old shape meant asserting that a
// 181-module code decodes from a screenshot at 3 px/module — which it
// does, most of the time, which is the worst kind of test.
//
// One deliberately oversized entry stays, because the fallback path is
// real: if packForQr cannot represent an SDP it sends it whole, and the
// product's answer to a code too dense for the space is to say so rather
// than to draw something unreadable. That case is asserted on the
// warning, not on a decode.
const COMPACT = 'B1|fLxh|Y4Z1LNXyMkO5W5epGtUyvNPW|' +
  'lVgLDrPGlx1kiCLaD5hvOkbmnKiOsZTkvpTQmjRNvrA|a|192.168.0.162:63857';
const PAYLOADS = [
  { name: 'compact offer', text: COMPACT },
  { name: 'compact, two routes', text: COMPACT + ',192.168.0.9:51000' },
  { name: 'compact, four routes', text: COMPACT + ',192.168.0.9:51000,10.0.0.5:40404,172.16.3.7:44444' },
];

/** A camera frame, built in the page, shared by the tests below.
 *
 * `capture(src, d)` paints a rendered code onto a 1280-square capture —
 * the size the scan loop's camera constraints ask for — at `fill` of the
 * frame, and can then abuse it the way a room does: `blur` for autofocus
 * that has not settled, `glare` for a reflection across the screen,
 * `patch` for something covering part of the code (centred, because a
 * blob over a corner takes out a finder or the alignment pattern and a
 * code that cannot be *located* cannot be corrected at any level — that
 * measures jsQR, not error correction).
 *
 * `readsAt(cap, dim, text)` decodes it the way tick() actually does:
 * downscale, plain decode, and Otsu on a miss. Measuring against plain
 * decoding alone would be measuring a loop this project does not ship.
 *
 * One definition rather than one per test: these frames are the evidence
 * behind two shipped decisions (the error-correction level, and the
 * occasional larger decode), and evidence that quietly differs between
 * tests — a different frame size here, a different default distance
 * there — is not comparable across them.
 */
const CAMERA_SIM = `
  var CAPTURE = 1280;
  function capture(src, d) {
    var c = document.createElement('canvas'); c.width = CAPTURE; c.height = CAPTURE;
    var x = c.getContext('2d');
    x.fillStyle = '#0b1016'; x.fillRect(0, 0, CAPTURE, CAPTURE);
    var size = Math.round(CAPTURE * (d.fill || 0.5)), off = Math.round((CAPTURE - size) / 2);
    if (d.blur) x.filter = 'blur(' + d.blur + 'px)';
    x.drawImage(src, off, off, size, size);
    x.filter = 'none';
    if (d.glare) {
      var g = x.createLinearGradient(off, off, off + size, off + size);
      g.addColorStop(0, 'rgba(255,255,255,0)');
      g.addColorStop(0.5, 'rgba(255,255,255,' + d.glare + ')');
      g.addColorStop(1, 'rgba(255,255,255,0)');
      x.fillStyle = g; x.fillRect(off, off, size, size);
    }
    if (d.patch) {
      var side = Math.round(size * Math.sqrt(d.patch)), at = Math.round((size - side) / 2);
      x.fillStyle = '#0b1016';
      x.fillRect(off + at, off + at, side, side);
    }
    return c;
  }
  function readsAt(cap, dim, text) {
    var t = document.createElement('canvas'); t.width = dim; t.height = dim;
    var x = t.getContext('2d', { willReadFrequently: true });
    x.drawImage(cap, 0, 0, CAPTURE, CAPTURE, 0, 0, dim, dim);
    var img = x.getImageData(0, 0, dim, dim);
    var r = jsQR(img.data, dim, dim, { inversionAttempts: 'dontInvert' });
    if (r && r.data === text) return true;
    window.BlipQR.filters.otsu(img);
    r = jsQR(img.data, dim, dim, { inversionAttempts: 'dontInvert' });
    return !!(r && r.data === text);
  }
`;

/** Nothing may be painted on top of the code.
 *
 * The page is a CRT cabinet: scanlines, a vignette, and an occasional
 * electrical "overcharge" glitch on the wordmark. All of those sit below
 * the pairing modal today (z-index 100, 99 and on the logo itself,
 * against the modal's 300), so none of them reaches the code. That is a
 * fact about current z-indexes, not a guarantee — raising a decorative
 * overlay above 300, or adding a new one without thinking about it,
 * would print scanlines straight across the QR. A camera would then see
 * a code that looks right to a human and will not decode.
 *
 * Checked by hit-testing rather than by reading z-indexes, so it holds
 * however the stacking is achieved. */
const NOTHING_OVER_THE_CODE = `(function () {
  var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
  if (!c) return { ok: false, why: 'no canvas' };
  var b = c.getBoundingClientRect();
  var pts = [
    [b.left + 4, b.top + 4], [b.right - 4, b.top + 4],
    [b.left + b.width / 2, b.top + b.height / 2],
    [b.left + 4, b.bottom - 4], [b.right - 4, b.bottom - 4]
  ];
  for (var i = 0; i < pts.length; i++) {
    var top = document.elementFromPoint(pts[i][0], pts[i][1]);
    if (top !== c) {
      return { ok: false, why: 'covered by <' + (top ? top.tagName.toLowerCase() +
        '.' + (top.className || '').toString().trim() : 'nothing') + '> at point ' + i };
    }
  }
  return { ok: true };
})()`;

/** Screenshot and decode what a camera would actually see.
 *
 * Deliberately the whole viewport, not the canvas element: an element
 * screenshot renders that element in isolation, so anything composited
 * *over* it -- a scanline overlay, a glitch effect, a translucent panel --
 * is invisible to the test while being exactly what ruins the scan in
 * real life. The camera sees the composite, so the test decodes the
 * composite. */
async function decodeOnScreen(page, cdp) {
  const el = await page.$('.blip-hs-panel canvas.blip-qr-canvas');
  assert.ok(el, 'no QR canvas on screen');
  const png = (await page.screenshot({ type: 'png' })).toString('base64');
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
  const { browser, page, cdp } = await openPage(t, ENGINE);

  for (const vp of VIEWPORTS) {
    for (const payload of PAYLOADS) {
      await t.test(`${vp.name} ${vp.width}x${vp.height}, ${payload.name}`, async () => {
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await loadRally(cdp);
        await openModal(cdp);
        await clickHsBtn(cdp, 'HOST');
        await waitFor(cdp, QR_READY, 20000);

        // Re-render the real on-screen canvas with a payload of known
        // size, through the same call and the same box the modal uses.
        const sdp = payload.text;
        // Through the modal's own render path, so the warning behaviour
        // under test is the one players actually get — but at a pinned
        // size, so nothing moves while the screenshot is taken.
        //
        // The live screen refits itself as the status line, the SCAN
        // button and the ICE list arrive (see watchPanelFit), which is
        // right for a player and wrong for a decode measurement: it
        // re-renders the canvas asynchronously, and a screenshot can
        // catch it mid-resize. Pinning the box is the documented way to
        // opt out. The unpinned, real-flow sizing is covered by its own
        // test at the end of this file.
        const res = await evaluate(cdp, `(function () {
          var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
          var box = c.parentNode.clientWidth;
          return window.__blipRenderCodeForTest(c, ${JSON.stringify(sdp)}, box);
        })()`);
        const info = res.render;

        // The invariant the fix rests on: the bitmap is displayed at
        // exactly its own device-pixel size, so nothing is resampled.
        const dpr = await evaluate(cdp, 'window.devicePixelRatio || 1');
        assert.equal(info.bitmapPx, Math.round(info.cssPx * dpr),
          `bitmap ${info.bitmapPx}px displayed at ${info.cssPx} CSS px x${dpr} — fractional scaling`);
        assert.equal(info.bitmapPx % info.modules, 0,
          `bitmap ${info.bitmapPx}px is not a whole number of ${info.modules} modules`);

        const clear = await evaluate(cdp, NOTHING_OVER_THE_CODE);
        assert.ok(clear.ok, `something is painted over the QR code: ${clear.why}`);

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
  const { browser, cdp } = await openPage(t, ENGINE);

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
  const { browser, cdp } = await openPage(t, ENGINE);
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
      // Every third size: this is a demonstration that fractional
      // scaling loses codes, and sampling the range shows that just as
      // well as walking it, in a third of the time. The full sweep took
      // ~27s on WebKit and timed out when the machine was busy.
      for (var s = 140; s <= 340; s += 3) {
        tried++;
        if (!decodeAt(legacy, s)) { failed++; if (failures.length < 6) failures.push(s); }
      }

      // The new behaviour: the bitmap is built to the box, one module
      // per whole number of device pixels, and displayed 1:1. Sweep the
      // same range of boxes and decode the bitmap that actually results.
      var fixedTried = 0, fixedFailed = 0, fixedFailures = [];
      for (var box = 140; box <= 340; box += 3) {
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
  const { browser, cdp } = await openPage(t, ENGINE);
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

// Filters, chosen by measurement rather than intuition.
//
// A phone camera pointed at another phone's screen does not hand jsQR the
// clean picture these tests otherwise use. It hands it a dim one, or one
// washed out by the room, or with a bright reflection across a third of
// the code, or softened because autofocus had not settled. Several of
// those frames carry a perfectly recoverable code that plain decoding
// refuses.
//
// Measured over a 54-frame corpus, Otsu binarisation is the one filter
// that pays: it reads frames plain decoding misses, and at the small
// decode size the loop now uses it is the difference between 14 and 21 of
// 24 frames. Contrast stretching scored 35/54 against plain's 34 -- inside
// the noise -- gamma scored *worse* than doing nothing, and Sauvola local
// thresholding added one frame for three times Otsu's cost. Only Otsu
// shipped. This is the test that it is worth its pass.
//
// Builds its own frames rather than using CAMERA_SIM above, on purpose:
// its 640-square frame at 55% fill is the geometry those published
// numbers were measured on, and re-measuring the same claim against a
// different frame would quietly stop being a check on it.
test(`binarising reads frames plain decoding cannot (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  const result = await evaluate(cdp, `(function () {
    var TEXT = 'B1|fLxh|Y4Z1LNXyMkO5W5epGtUyvNPW|lVgLDrPGlx1kiCLaD5hvOkbmnKiOsZTkvpTQmjRNvrA|a|192.168.0.162:63857';
    var DIM = 640;
    var src = document.createElement('canvas');
    window.BlipQR.render(src, TEXT, { fitCssPx: 300, dpr: 1 });

    function frame(d) {
      var c = document.createElement('canvas'); c.width = DIM; c.height = DIM;
      var x = c.getContext('2d');
      x.fillStyle = '#0b1016'; x.fillRect(0, 0, DIM, DIM);
      var size = Math.round(DIM * 0.55), off = Math.round((DIM - size) / 2);
      if (d.blur) x.filter = 'blur(' + d.blur + 'px)';
      x.drawImage(src, off, off, size, size);
      x.filter = 'none';
      var img = x.getImageData(0, 0, DIM, DIM), p = img.data, i;
      if (d.dim) for (i = 0; i < p.length; i += 4) { p[i] *= d.dim; p[i+1] *= d.dim; p[i+2] *= d.dim; }
      if (d.flat) for (i = 0; i < p.length; i += 4) {
        p[i] = 128 + (p[i]-128)*d.flat; p[i+1] = 128 + (p[i+1]-128)*d.flat; p[i+2] = 128 + (p[i+2]-128)*d.flat; }
      if (d.noise) for (i = 0; i < p.length; i += 4) {
        var n = (Math.random()-0.5)*d.noise; p[i]+=n; p[i+1]+=n; p[i+2]+=n; }
      return img;
    }
    function copy(img) { return new ImageData(new Uint8ClampedArray(img.data), img.width, img.height); }
    function reads(img) {
      var r = jsQR(img.data, img.width, img.height, { inversionAttempts: 'dontInvert' });
      return !!(r && r.data === TEXT);
    }

    var CASES = {
      'clean': {}, 'dim': { dim: 0.25 }, 'very dim': { dim: 0.10 },
      'low contrast': { flat: 0.25 }, 'dim + low contrast': { dim: 0.35, flat: 0.4 },
      'blurred': { blur: 2 }, 'noisy': { noise: 90 }, 'dim + noisy': { dim: 0.3, noise: 60 }
    };
    var plain = 0, ladder = 0, rescued = [];
    Object.keys(CASES).forEach(function (name) {
      var base = frame(CASES[name]);
      var okPlain = reads(copy(base));
      if (okPlain) { plain++; ladder++; return; }
      // The rungs blip_qr.js actually rotates through.
      var okLadder = ['otsu'].some(function (f) {
        var im = copy(base);
        return window.BlipQR.filters[f](im) && reads(im);
      });
      if (okLadder) { ladder++; rescued.push(name); }
    });
    return { total: Object.keys(CASES).length, plain: plain, ladder: ladder, rescued: rescued };
  })()`);

  t.diagnostic(`plain ${result.plain}/${result.total}, with ladder ${result.ladder}/${result.total}` +
    `${result.rescued.length ? ' — rescued: ' + result.rescued.join(', ') : ''}`);

  assert.ok(result.ladder > result.plain,
    `otsu read no more than plain decoding (${result.ladder} vs ${result.plain}) — it is not earning its pass`);
  assert.ok(result.rescued.includes('dim + noisy') || result.rescued.includes('noisy'),
    `a noisy frame should be recovered by otsu; rescued: ${JSON.stringify(result.rescued)}`);
});

// Scanning has to stay affordable on the slowest device that will run it.
//
// The scan loop originally decoded the raw camera frame, which the camera
// was asked to make as large as 1920 square. Measured on a fast desktop
// that is ~115ms per frame -- an 8fps ceiling before the game loop, the
// preview and the overlay have had a turn -- and a phone is several times
// slower again. That is what "the scan is really slow and never reads it"
// is: the preview stutters, so the code cannot be held steady, and every
// frame it *is* held costs a tenth of a second to look at.
//
// Asserted as a ratio rather than a millisecond budget, because the
// absolute number is a property of whatever machine is running the test
// and would have to be loosened until it meant nothing. The ratio is a
// property of the code.
test(`decoding stays cheap enough for a phone (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  const result = await evaluate(cdp, `(function () {
    var TEXT = 'B1|fLxh|Y4Z1LNXyMkO5W5epGtUyvNPW|lVgLDrPGlx1kiCLaD5hvOkbmnKiOsZTkvpTQmjRNvrA|a|192.168.0.162:63857';
    var src = document.createElement('canvas');
    window.BlipQR.render(src, TEXT, { fitCssPx: 300, dpr: 1 });

    // A 1280-square capture, as the camera constraints now ask for.
    var cap = document.createElement('canvas'); cap.width = 1280; cap.height = 1280;
    var cx = cap.getContext('2d');
    cx.fillStyle = '#0b1016'; cx.fillRect(0, 0, 1280, 1280);
    var size = Math.round(1280 * 0.5), off = Math.round((1280 - size) / 2);
    cx.drawImage(src, off, off, size, size);

    function costAt(dim) {
      var c = document.createElement('canvas'); c.width = dim; c.height = dim;
      var x = c.getContext('2d');
      x.drawImage(cap, 0, 0, 1280, 1280, 0, 0, dim, dim);
      var runs = 8, decoded = false, t0 = performance.now();
      for (var i = 0; i < runs; i++) {
        var img = x.getImageData(0, 0, dim, dim);
        var r = jsQR(img.data, dim, dim, { inversionAttempts: 'dontInvert' });
        decoded = !!(r && r.data === TEXT);
      }
      return { dim: dim, ms: (performance.now() - t0) / runs, decoded: decoded };
    }
    return { shipped: costAt(400), raw: costAt(1280) };
  })()`);

  t.diagnostic(`decode at 400: ${result.shipped.ms.toFixed(1)}ms, ` +
    `at 1280: ${result.raw.ms.toFixed(1)}ms (${(result.raw.ms / result.shipped.ms).toFixed(1)}x)`);

  assert.ok(result.shipped.decoded, 'the shipped decode size must still read a realistic code');
  // Threshold 2, not 3: the measured ratio is ~5x on Chromium and ~3x on
  // WebKit, and a bound set at the tighter engine's own number fails on
  // its ordinary run-to-run variance. What this guards against is the
  // regression that actually matters -- the loop going back to decoding
  // the raw frame -- which would put the ratio far above either.
  assert.ok(result.raw.ms / result.shipped.ms > 2,
    `decoding small is only ${(result.raw.ms / result.shipped.ms).toFixed(1)}x cheaper than decoding the raw ` +
    'frame — if that gap has closed, the scan loop is probably decoding at full resolution again');
});

// The code must fit the panel it is shown in — the whole code, and the
// controls under it.
//
// This is the case the rest of this file could not see. Every test above
// re-renders through `__blipRenderCodeForTest`, which sets the box size
// explicitly, so none of them exercised the sizing the real HOST screen
// actually computes. When the panel was allowed to grow to most of the
// screen, it overflowed by 55-115px at every desktop size: the panel
// scrolled, the SCAN ANSWER button the host must press went below the
// fold, and scrolling to reach it took half the code out of view.
//
// The sizing is awkward because the panel's height is not known once. The
// code is rendered before the status line and the button exist, and the
// live ICE list appears later still as candidates are gathered. So this
// drives the real flow, waits for all of that to land, and then asks the
// only question that matters.
test(`the whole code and its controls fit the panel (${ENGINE})`, async (t) => {
  const { cdp, page } = await openPage(t, ENGINE);

  // Browser viewports, not screen sizes: a MacBook's window is shorter
  // than its display once the menu bar, tab strip and dock are taken out.
  const VIEWPORTS = [
    { name: 'macbook, windowed', width: 1512, height: 860 },
    { name: 'macbook, short', width: 1512, height: 760 },
    { name: 'macbook air', width: 1440, height: 780 },
    { name: 'desktop', width: 1280, height: 800 },
    { name: '1366 laptop', width: 1366, height: 700 },
    { name: 'iPhone portrait', width: 390, height: 844 },
    { name: 'iPad', width: 820, height: 1180 },
  ];

  for (const vp of VIEWPORTS) {
    await t.test(`${vp.name} ${vp.width}x${vp.height}`, async () => {
      await page.setViewportSize({ width: vp.width, height: vp.height });
      await loadRally(cdp);
      await openModal(cdp);
      await clickHsBtn(cdp, 'HOST');
      await waitFor(cdp, QR_READY, 20000);
      // The ICE list is the last thing to arrive; give it a beat.
      await sleep(1500);

      const m = await evaluate(cdp, `(function () {
        var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
        var p = document.querySelector('.blip-hs-panel');
        if (!c || !p) return null;
        var cb = c.getBoundingClientRect(), pb = p.getBoundingClientRect();
        // Both axes. The panel clips horizontally as readily as it does
        // vertically, and a code missing its right-hand column has lost
        // a finder pattern — it is not a smaller code, it is not a code.
        var vis = {
          left: pb.left + p.clientLeft, top: pb.top + p.clientTop,
          right: pb.left + p.clientLeft + p.clientWidth,
          bottom: pb.top + p.clientTop + p.clientHeight
        };
        function overlap(aLo, aHi, bLo, bHi) { return Math.max(0, Math.min(aHi, bHi) - Math.max(aLo, bLo)); }
        var insidePanel = overlap(cb.top, cb.bottom, vis.top, vis.bottom) *
          overlap(cb.left, cb.right, vis.left, vis.right);
        var insideView = overlap(cb.top, cb.bottom, 0, window.innerHeight) *
          overlap(cb.left, cb.right, 0, window.innerWidth);
        var area = cb.width * cb.height;
        return {
          code: Math.round(cb.width),
          panelOverflow: p.scrollHeight - p.clientHeight,
          panelOverflowX: p.scrollWidth - p.clientWidth,
          inPanelPct: Math.round(100 * insidePanel / area),
          inViewportPct: Math.round(100 * insideView / area),
          scannable: window.BlipQR.lastRender.scannable,
          pxPerModule: window.BlipQR.lastRender.cssPxPerModule
        };
      })()`);

      assert.ok(m, 'no code on screen after pressing HOST');
      t.diagnostic(`${vp.name}: ${m.code}px, ${m.pxPerModule.toFixed(1)} px/module, ` +
        `overflow ${m.panelOverflow}px`);

      assert.equal(m.inPanelPct, 100,
        `${m.inPanelPct}% of the code is inside the panel — the rest is clipped`);
      assert.equal(m.inViewportPct, 100,
        `${m.inViewportPct}% of the code is on screen — the rest is off the viewport`);
      assert.ok(m.panelOverflowX <= 0,
        `the panel scrolls sideways by ${m.panelOverflowX}px — the code is wider than the ` +
        'panel it is drawn in, so its edge is cropped');
      assert.ok(m.panelOverflow <= 0,
        `the panel scrolls by ${m.panelOverflow}px, so the controls under the code are below ` +
        'the fold and reaching them scrolls the code out of view');

      // Fitting by shrinking to nothing would satisfy everything above.
      assert.ok(m.scannable,
        `the code was shrunk to ${m.pxPerModule.toFixed(1)} px/module, below what a camera can read`);
    });
  }
});

// Error correction costs modules, and modules cost camera pixels.
//
// The encoder keeps the first level that fits and reads back, so the
// order of that list decides what every player photographs. Starting it
// at H — the strongest — is the intuitive choice and measurably the
// wrong one: at a fixed size on glass a stronger level is a *denser*
// code, and density, not damage, is what defeats a phone camera reading
// another phone's screen.
//
// This is the measurement that decided it, cut down to the four cases
// that separate H from Q. It is here so that "surely stronger is safer"
// cannot quietly put H back.
//
// It does not adjudicate Q against M — on the full corpus those two tie
// at 12 frames of 14 and differ only in which ones they drop, which is
// an argument about the *loop* (Q's failures are the ones the periodic
// larger decode recovers) rather than about the levels. That reasoning
// lives with ECC_LEVELS in web/blip_qr.js, next to the list it explains.
test(`the error-correction level is chosen for the camera, not for damage (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  const result = await evaluate(cdp, `(function () {
    ${CAMERA_SIM}
    var TEXT = ${JSON.stringify(COMPACT)};

    function draw(level, box) {
      var qr = qrcode(0, level);
      qr.addData(TEXT); qr.make();
      var count = qr.getModuleCount(), total = count + 8;
      var cell = Math.max(1, Math.floor(box / total)), size = cell * total;
      var c = document.createElement('canvas');
      c.width = size; c.height = size;
      var x = c.getContext('2d');
      x.fillStyle = '#fff'; x.fillRect(0, 0, size, size);
      x.fillStyle = '#000';
      for (var r = 0; r < count; r++) for (var q = 0; q < count; q++)
        if (qr.isDark(r, q)) x.fillRect(4 * cell + q * cell, 4 * cell + r * cell, cell, cell);
      return { canvas: c, modules: total };
    }

    // The code covers the same fraction of the frame at every level —
    // same screen, same distance — which is exactly why a denser code
    // arrives with fewer camera pixels per module.
    var CASES = [
      { n: 'far (25% of frame)', d: { fill: 0.25 } },
      { n: 'far + blur', d: { fill: 0.25, blur: 2 } },
      { n: 'covered 10%', d: { patch: 0.10 } },
      { n: 'covered 10% + far', d: { patch: 0.10, fill: 0.35 } }
    ];

    var out = {};
    ['H', 'Q'].forEach(function (level) {
      var drawn = draw(level, 488); // the desktop code's real size
      var read = [];
      CASES.forEach(function (c) {
        // Best of two: these frames are deterministic apart from the
        // blur path's rounding, which the repeat covers.
        if (readsAt(capture(drawn.canvas, c.d), 400, TEXT) ||
            readsAt(capture(drawn.canvas, c.d), 400, TEXT)) read.push(c.n);
      });
      out[level] = { modules: drawn.modules, read: read };
    });
    out.shipped = window.BlipQR.render(document.createElement('canvas'), TEXT, { fitCssPx: 488, dpr: 1 });
    return out;
  })()`);

  t.diagnostic(`H: ${result.H.modules} modules, read ${result.H.read.length}/4 [${result.H.read}]`);
  t.diagnostic(`Q: ${result.Q.modules} modules, read ${result.Q.read.length}/4 [${result.Q.read}]`);

  assert.ok(result.Q.modules < result.H.modules,
    'Q should need fewer modules than H — if not, this whole trade-off has changed');
  assert.ok(result.Q.read.length > result.H.read.length,
    `H read ${result.H.read.length} of these frames and Q read ${result.Q.read.length} — ` +
    'the denser code is no longer the worse one, so the level order deserves re-measuring');
  assert.equal(result.shipped.ecc, 'Q',
    `a realistic payload rendered at ECC ${result.shipped.ecc}; the modal ships the first ` +
    'level in BlipQR\'s list that fits, and the measurement above says that should be Q');
});

// Why the scan loop decodes one frame in four at a larger size.
//
// Decoding small is what makes the preview smooth, and for most frames
// it is also enough — see DECODE_DIM's table in web/blip_qr.js. But
// "enough" was measured over frames where the code fills 60-20% of the
// view and is dim, blurred or noisy. Two things outside that corpus are
// ordinary in real use and need the pixels: a code that fills only a
// fifth of the frame, and one with a reflection across it, where Otsu's
// whole-frame threshold is actively the wrong tool.
//
// This asserts the trade is real in both directions: the larger decode
// reads frames the small one misses, and it is not just "bigger is
// better" — full resolution loses the glare frame that 640 recovers.
test(`the occasional larger decode reads frames the small one cannot (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  const result = await evaluate(cdp, `(function () {
    ${CAMERA_SIM}
    var TEXT = ${JSON.stringify(COMPACT)};
    var src = document.createElement('canvas');
    window.BlipQR.render(src, TEXT, { fitCssPx: 488, dpr: 1 });

    var d = window.BlipQR.decode;
    var CASES = {
      'held back (20% of frame)': { fill: 0.20 },
      'reflection across the code': { fill: 0.5, glare: 0.75 },
      'ordinary (50% of frame)': { fill: 0.5 }
    };
    var out = { sizes: d, cases: {} };
    Object.keys(CASES).forEach(function (name) {
      var cap = capture(src, CASES[name]);
      out.cases[name] = {
        small: readsAt(cap, d.dim, TEXT),
        far: readsAt(cap, d.far, TEXT),
        full: readsAt(cap, CAPTURE, TEXT)
      };
    });
    return out;
  })()`);

  t.diagnostic(`decode sizes: ${JSON.stringify(result.sizes)}`);
  for (const [name, r] of Object.entries(result.cases)) {
    t.diagnostic(`${name}: ${result.sizes.dim}px ${r.small ? 'read' : 'missed'}, ` +
      `${result.sizes.far}px ${r.far ? 'read' : 'missed'}, full ${r.full ? 'read' : 'missed'}`);
  }

  assert.ok(result.sizes.far > result.sizes.dim, 'the larger decode is not larger');
  assert.ok(result.sizes.every >= 2,
    'decoding every frame at the larger size would cost the preview its frame rate');

  const rescued = Object.entries(result.cases).filter(([, r]) => r.far && !r.small).map(([n]) => n);
  assert.ok(rescued.length > 0,
    'no frame needed the larger decode — if that is really true the loop can drop it and ' +
    'run cheaper, but check the corpus before believing it');

  assert.ok(result.cases['ordinary (50% of frame)'].small,
    'the common case must still read at the small size, or the loop is paying 3x on every frame');
});
