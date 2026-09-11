// End-to-end test for two-device Rally (docs/multiplayer.md): QR-code
// pairing is the *only* signaling path (no Realtime relay, no network of
// any kind involved in pairing), so this test proves the real thing a
// player does — point a camera at the other phone's screen — actually
// works, not just that the SDP math round-trips.
//
// Chrome can feed a specific video file into `getUserMedia` in headless
// mode (`--use-fake-device-for-media-stream
// --use-file-for-fake-video-capture=<file>.y4m`). This test renders each
// side's real QR code, pipes it through ffmpeg into a video file, and
// points the *other* side's fake camera at that file — so the guest's
// JOIN literally opens a (fake) camera, samples (fake) video frames, and
// decodes a real QR code with jsQR, and likewise for the host scanning
// the answer back. The only thing not exercised is a physical lens
// pointed at a physical screen; every line of code between `getUserMedia`
// and a decoded SDP string runs for real.
//
// Requires: `chromium` (or `$BLIP_CHROMIUM`) and `ffmpeg` on PATH, and
// ./build_web.sh already having produced web/rally/index.wasm.
//
// Run: npm run test:multiplayer

import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { connect, evaluate, waitFor, sleep, killAll } from './lib/cdp.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEB_DIR = path.join(__dirname, '..', 'web');
const PORT = 8098; // distinct from the 8080 devs run by hand, so both can coexist
const HOST_DEBUG_PORT = 9531;
const GUEST_DEBUG_PORT = 9532;

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

let server;
let procs = [];
let tmpDir;

function sh(cmd, args) {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: 'ignore' });
    p.on('error', reject);
    p.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} exited ${code}`))));
  });
}

/** A short-lived, single-frame-looped video ffmpeg can build from any
 * still image — used both for the initial placeholder (before a real QR
 * exists) and for the QR frames themselves. Padded onto a plain white
 * field with a real quiet-zone margin around the code, the way a phone
 * held up to a screen would actually frame it — jsQR needs that light
 * border around the finder patterns to detect them reliably. */
async function writeQrVideo(pngPath, outPath) {
  // The PNG is exactly what web/blip_qr.js's render() draws — quiet zone
  // included, no extra help from this test. The dark pad around it (the
  // pairing modal's own panel color, not white) is what a camera aimed at
  // a real phone actually sees surrounding the code: the code's own quiet
  // zone against the modal background, not a page of white paper.
  await sh('ffmpeg', ['-y', '-loop', '1', '-i', pngPath,
    '-vf', 'scale=440:440:flags=neighbor,pad=480:480:(ow-iw)/2:(oh-ih)/2:#0b1016',
    '-t', '8', '-pix_fmt', 'yuv420p', outPath]);
}
async function writeBlankVideo(outPath) {
  await sh('ffmpeg', ['-y', '-f', 'lavfi', '-i', 'color=white:s=480x480:d=2', '-pix_fmt', 'yuv420p', outPath]);
}

async function launchWithCamera(port, camFile) {
  const bin = process.env.BLIP_CHROMIUM || 'chromium';
  const proc = spawn(bin, [
    '--headless=new', '--disable-gpu', '--no-sandbox', '--disable-dev-shm-usage',
    '--disable-features=WebRtcHideLocalIpsWithMdns',
    '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding', '--disable-background-timer-throttling',
    '--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream', // auto-grant the camera prompt
    `--use-file-for-fake-video-capture=${camFile}`,
    `--remote-debugging-port=${port}`, '--js-flags=--max-old-space-size=192',
    'about:blank',
  ], { stdio: 'ignore' });
  const cdp = await connect(port);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  return { proc, cdp };
}

before(async () => {
  tmpDir = await import('node:fs/promises').then((fs) => fs.mkdtemp(path.join(os.tmpdir(), 'blip-mp-')));
  server = createServer(async (req, res) => {
    try {
      let p = decodeURIComponent(req.url.split('?')[0]);
      if (p.endsWith('/')) p += 'index.html';
      const full = path.join(WEB_DIR, p);
      if (!full.startsWith(WEB_DIR)) { res.writeHead(403); res.end(); return; }
      const data = await readFile(full);
      res.writeHead(200, { 'Content-Type': MIME[path.extname(full)] || 'application/octet-stream' });
      res.end(data);
    } catch (e) {
      res.writeHead(404);
      res.end('not found');
    }
  });
  await new Promise((resolve) => server.listen(PORT, '127.0.0.1', resolve));
});

after(async () => {
  killAll(procs);
  await new Promise((resolve) => server.close(resolve));
});

/** The dial hand's rotation encodes the paddle fraction — see
 * shell.js's `window.blipPaddles`. Reading it back from the live DOM
 * (rather than trying to intercept the callback, which races against
 * shell.js's own unconditional `window.blipPaddles = ...` assignment)
 * gives an injection-free way to observe *both* sides' idea of the right
 * paddle's position — host (simulated) and guest (mirrored from the
 * network) — without any test-only hook in the game or bridge code. */
const READ_RIGHT_FRACTION = `(function () {
  var el = document.getElementById('dial-hand-p2');
  if (!el) return null;
  var m = /rotate\\(([-\\d.eE]+)rad\\)/.exec(el.style.transform || '');
  if (!m) return null;
  var SWEEP = 3 * Math.PI;
  return parseFloat(m[1]) / SWEEP + 0.5;
})()`;

// A rendered QR canvas defaults to HTML's stock 300x150 before
// window.BlipQR.render() ever runs — check for a *square*, non-default
// size so "ready" actually means "has a real code in it", not just
// "the element exists".
const QR_READY = `(function(){
  var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
  return !!(c && c.width > 50 && c.width === c.height);
})()`;

async function loadRally(cdp) {
  await evaluate(cdp, 'true'); // ensure Runtime is ready before navigating
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${PORT}/rally/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipNet === 'object' && typeof window.BlipQR === 'object'", 15000);
  // The coin wall: shell.js swallows *every* keydown at the window
  // capture phase while it's up (`overlay.classList.contains('visible')`
  // -> stopImmediatePropagation — see web/shell.js). Dismiss it the same
  // way a real player would: a click/tap on the overlay inserts a coin.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await evaluate(cdp, "document.getElementById('need-coin-overlay').click()");
  }
}

async function grabQrPng(cdp, outPath) {
  const b64 = await evaluate(cdp, "document.querySelector('.blip-hs-panel canvas.blip-qr-canvas').toDataURL('image/png').split(',')[1]");
  await writeFile(outPath, Buffer.from(b64, 'base64'));
}

test('two-device Rally: QR-only pairing, real camera decode both directions, input+state sync', async (t) => {
  const hostCam = path.join(tmpDir, 'host_cam.y4m');
  const guestCam = path.join(tmpDir, 'guest_cam.y4m');
  await Promise.all([writeBlankVideo(hostCam), writeBlankVideo(guestCam)]);

  const hostChrome = await launchWithCamera(HOST_DEBUG_PORT, hostCam);
  const guestChrome = await launchWithCamera(GUEST_DEBUG_PORT, guestCam);
  procs = [hostChrome.proc, guestChrome.proc];
  const host = hostChrome.cdp;
  const guest = guestChrome.cdp;

  await Promise.all([loadRally(host), loadRally(guest)]);

  await t.test('host generates an offer and renders it as a QR code', async () => {
    await evaluate(host, "document.getElementById('net-play-btn').click()");
    await evaluate(host, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent==='HOST';}).click()");
    await waitFor(host, QR_READY, 8000);
    await grabQrPng(host, path.join(tmpDir, 'offer.png'));
    await writeQrVideo(path.join(tmpDir, 'offer.png'), guestCam);
  });

  await t.test('guest scans the offer with its (fake) camera and renders an answer QR', async () => {
    await evaluate(guest, "document.getElementById('net-play-btn').click()");
    await evaluate(guest, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent==='JOIN';}).click()");
    // JOIN auto-starts the camera scan (see blip_net_ui.js's showJoinQR) —
    // no button to click, just wait for either a decode or an error.
    const outcome = await (async () => {
      for (let i = 0; i < 100; i++) {
        const err = await evaluate(guest, "(document.querySelector('.blip-hs-err')||{}).textContent || ''");
        if (err) return { ok: false, err };
        if (await evaluate(guest, QR_READY)) return { ok: true };
        await sleep(200);
      }
      return { ok: false, err: 'timed out waiting for a decode' };
    })();
    assert.ok(outcome.ok, `guest never decoded the offer QR: ${outcome.err}`);
    await grabQrPng(guest, path.join(tmpDir, 'answer.png'));
    await writeQrVideo(path.join(tmpDir, 'answer.png'), hostCam);
  });

  await t.test('host scans the answer with its (fake) camera and the DataChannel opens on both sides', async () => {
    await evaluate(host, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent.indexOf('SCAN')!==-1;}).click()");
    const hostRole = await waitFor(host, 'window.blipNetRole()', 15000);
    const guestRole = await waitFor(guest, 'window.blipNetRole()', 15000);
    assert.equal(hostRole, 1, 'host should report role 1 once its DataChannel opens');
    assert.equal(guestRole, 2, 'guest should report role 2 once its DataChannel opens');
  });

  await t.test('the match actually starts on both devices (Rust picks up the net role)', async () => {
    const hostFrac = await waitFor(host, `(${READ_RIGHT_FRACTION}) > 0.3`, 10000);
    const guestFrac = await waitFor(guest, `(${READ_RIGHT_FRACTION}) > 0.3`, 10000);
    assert.ok(hostFrac, 'host paddle never left the Title-screen position');
    assert.ok(guestFrac, 'guest paddle never left the Title-screen position');
  });

  await t.test('guest input reaches the host and the resulting state syncs back', async () => {
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.3 && before < 0.7, `expected a centered paddle to start, got ${before}`);

    // Simulate the guest's on-screen P2 dial being spun "up" — exactly
    // the KeyboardEvent bindDial() would dispatch on a real touch, which
    // is what blip_net.js's guest input capture is listening for.
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);

    // Wait for the guest's own mirrored view to show real movement first
    // (proves guest -> host -> guest made at least one full round trip),
    // then RELEASE immediately before comparing host vs guest — both
    // paddles are in continuous motion the whole time the key is held, so
    // a snapshot taken mid-motion catches the two sides at slightly
    // different points along that motion (one DataChannel round trip
    // apart) and is a flaky comparison by construction.
    //
    // Kept deliberately fast for a second reason: any key press also
    // satisfies Rally's own any_key_pressed() launch condition on the
    // real Serve state it's in — the ball goes live, and if it actually
    // scores, reset_for_serve() recenters both paddles regardless of what
    // this test is doing. Rally's ball can't cross the ~450px field in
    // under ~0.8s even at the sharpest launch angle, so finishing this
    // whole check well inside that window is what keeps it a non-issue.
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 8000);
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);
    await sleep(150); // let the last couple of state packets in flight land on both sides

    const guestAfter = await evaluate(guest, READ_RIGHT_FRACTION);
    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(guestAfter < before,
      `guest's own mirrored paddle should show it moved up (was ${before}, now ${guestAfter})`);
    assert.ok(hostAfter < before,
      `host should have actually simulated the remote input (was ${before}, now ${hostAfter})`);
    assert.ok(Math.abs(guestAfter - hostAfter) < 0.05,
      `host and guest should agree on the settled paddle position (host ${hostAfter}, guest ${guestAfter})`);

    // Should have actually stopped — not still drifting from a keyup
    // that never latched. A jump to ~0.5 here would be reset_for_serve()
    // recentering both paddles because the point actually finished
    // (see the timing note above) — vanishingly unlikely inside this
    // ~150ms window, but not a networking failure if it happens.
    const settled1 = await evaluate(guest, READ_RIGHT_FRACTION);
    await sleep(150);
    const settled2 = await evaluate(guest, READ_RIGHT_FRACTION);
    const looksLikeAPointReset = Math.abs(settled2 - 0.5) < 0.02 && Math.abs(settled1 - settled2) > 0.05;
    if (looksLikeAPointReset) {
      t.diagnostic(`paddle snapped to ~0.5 between settle reads (${settled1} -> ${settled2}) — ` +
        'the ball most likely scored and reset_for_serve() ran; not treating as a sync failure');
    } else {
      assert.ok(Math.abs(settled1 - settled2) < 0.01,
        `paddle should have stopped moving after keyup (${settled1} -> ${settled2})`);
    }
  });

  await t.test('a clean disconnect is observed on both sides', async () => {
    await evaluate(host, 'window.BlipNet.cancel()');
    const hostRole = await waitFor(host, 'window.blipNetRole() === 0', 5000);
    assert.equal(hostRole, true);
    // The guest's channel closing is a consequence of the host's peer
    // connection tearing down — allow a little longer for that to land.
    const guestRole = await waitFor(guest, 'window.blipNetRole() === 0', 8000);
    assert.equal(guestRole, true);
  });
});
