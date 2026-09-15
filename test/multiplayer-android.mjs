// Single-*Android-emulator* end-to-end test for Rally pairing
// (docs/multiplayer.md, docs/android-multiplayer-test-plan.md).
//
// test/multiplayer.mjs already proves the pairing logic itself against
// two headless Chromium instances with Chrome's own fake-camera flags.
// This test proves the same flow against real Chrome-for-Android — a
// different WebView/renderer, different touch/permission plumbing, no
// `--use-fake-device-for-media-stream` flag available at all — by
// opening the host and guest in two independent Android emulators.
//
// Two independent renderers are required for the gameplay assertions:
// Chrome may throttle a background tab even while its DataChannel remains
// open, which can make pairing pass while the WASM game loop is stopped.
//
// This also does NOT use the emulator's camera at all. `-camera-back
// imagefile:<path>` (the kernel-module-free mechanism designed in
// docs/android-multiplayer-test-plan.md) turned out not to pass image
// content through faithfully: measured with a calibration image (colored
// corner markers on an otherwise-blank canvas), the emulator's virtual
// camera pipeline applies some undocumented crop/zoom/rotate transform
// before Chrome ever sees a frame, so a QR code fed in that way doesn't
// come out scannable. See docs/android-multiplayer-test-plan.md for the
// full investigation.
//
// Instead, web/blip_qr.js exposes a test-only hook (inert for real
// players): `window.BlipQR.testInject = <text>` makes the next scan()
// resolve with that text immediately instead of opening the camera, and
// `window.BlipQR.lastRenderedText` records the exact string the other
// side's QR would have encoded. That lets this test drive the real
// WebRTC signaling/role-assignment/state-sync logic end-to-end on real
// Chrome-for-Android — the part that's actually different from the
// desktop test — while treating "a device's camera can physically
// photograph a QR code off another screen" as already covered by the
// desktop test's real jsQR decode over a real (v4l2loopback-backed) fake
// camera.
//
// Requires: KVM (`/dev/kvm`), and the Android SDK/emulator (installed
// automatically into $ANDROID_SDK_ROOT, default ~/Android/Sdk, on first
// run — see scripts/lib/android-emulator.sh). Skips itself (rather than
// failing) when neither is available, so `npm test` stays green on a
// machine that never asked for this.
//
// Run: npm run test:multiplayer:android

import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { evaluate, waitFor, sleep, connectAndroid } from './lib/cdp.mjs';
import * as avd from './lib/android-emulator.mjs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEB_DIR = path.join(__dirname, '..', 'web');
const REPORT_DIR = path.join(__dirname, '..', 'docs', 'images', 'android-multiplayer');
const PORT = 8099; // distinct from dev (8080) and the desktop test (8098)

const HOST_AVD = 'blip_host';
const GUEST_AVD = 'blip_guest';
const HOST_PORT = 5554;
const GUEST_PORT = 5556;
const HOST_SERIAL = `emulator-${HOST_PORT}`;
const GUEST_SERIAL = `emulator-${GUEST_PORT}`;
const HOST_CDP_PORT = 9541;
const GUEST_CDP_PORT = 9542;

// Two different URLs (not two devices) give the host and guest each
// their own Chrome tab — `am start` on an already-running URL just
// refocuses the existing tab instead of opening a new one, so the query
// string also doubles as how connectAndroid's urlIncludes tells the two
// tabs' CDP targets apart.
const HOST_URL = `http://127.0.0.1:${PORT}/rally/index.html?role=host`;
const GUEST_URL = `http://127.0.0.1:${PORT}/rally/index.html?role=guest`;

// Skip (not fail) on a machine that can't run this: no KVM means the
// emulator would run unusably slowly (or not at all) via pure software
// emulation, and this is an opt-in local/CI-with-nested-virt tier, not
// part of the default `npm test` run.
const canRun = process.platform === 'linux' && existsSync('/dev/kvm');
const skipReason = canRun ? false : 'requires Linux + /dev/kvm (see docs/android-multiplayer-test-plan.md)';

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

let server;
let tmpDir;

before(async () => {
  if (!canRun) return;
  tmpDir = await (await import('node:fs/promises')).mkdtemp(path.join(os.tmpdir(), 'blip-mp-android-'));

  server = createServer(async (req, res) => {
    try {
      let p = decodeURIComponent(req.url.split('?')[0]);
      if (p.endsWith('/')) p += 'index.html';
      const full = path.join(WEB_DIR, p);
      if (!full.startsWith(WEB_DIR)) { res.writeHead(403); res.end(); return; }
      const data = await readFile(full);
      res.writeHead(200, { 'Content-Type': MIME[path.extname(full)] || 'application/octet-stream' });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end('not found');
    }
  });
  await new Promise((resolve) => server.listen(PORT, '127.0.0.1', resolve));

  await avd.ensureSdk();
  await Promise.all([
    avd.createAvdIfMissing(HOST_AVD),
    avd.createAvdIfMissing(GUEST_AVD),
  ]);
  await Promise.all([
    avd.boot(HOST_AVD, HOST_PORT, path.join(tmpDir, 'host-android.log')),
    avd.boot(GUEST_AVD, GUEST_PORT, path.join(tmpDir, 'guest-android.log')),
  ]);
  await Promise.all([
    avd.waitForBoot(HOST_SERIAL),
    avd.waitForBoot(GUEST_SERIAL),
  ]);
  await Promise.all([
    avd.skipSetupWizard(HOST_SERIAL),
    avd.skipSetupWizard(GUEST_SERIAL),
    avd.reversePort(HOST_SERIAL, PORT),
    avd.reversePort(GUEST_SERIAL, PORT),
  ]);
  await Promise.all([
    avd.resetChrome(HOST_SERIAL),
    avd.resetChrome(GUEST_SERIAL),
  ]);
  await Promise.all([
    avd.openUrlDismissOnboarding(HOST_SERIAL, HOST_URL),
    avd.openUrlDismissOnboarding(GUEST_SERIAL, GUEST_URL),
  ]);
  await Promise.all([
    avd.forwardDevtools(HOST_SERIAL, HOST_CDP_PORT),
    avd.forwardDevtools(GUEST_SERIAL, GUEST_CDP_PORT),
  ]);
});

after(async () => {
  if (canRun) {
    await Promise.allSettled([
      avd.kill(HOST_SERIAL),
      avd.kill(GUEST_SERIAL),
    ]);
  }
  if (server) await new Promise((resolve) => server.close(resolve));
});

// Same read-back trick test/multiplayer.mjs uses: the dial hand's
// rotation encodes the paddle fraction (window.blipPaddles in shell.js).
const READ_RIGHT_FRACTION = `(function () {
  var el = document.getElementById('dial-hand-p2');
  if (!el) return null;
  var m = /rotate\\(([-\\d.eE]+)rad\\)/.exec(el.style.transform || '');
  if (!m) return null;
  var SWEEP = 3 * Math.PI;
  return parseFloat(m[1]) / SWEEP + 0.5;
})()`;

const QR_READY = `(function(){
  var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
  return !!(c && c.width > 50 && c.width === c.height);
})()`;

async function loadRally(cdp) {
  await waitFor(cdp, "document.readyState === 'complete'", 20000);
  await waitFor(cdp, "typeof window.BlipNet === 'object' && typeof window.BlipQR === 'object'", 20000);
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await evaluate(cdp, "document.getElementById('need-coin-overlay').click()");
  }
}

/** Read back the exact text the QR canvas currently on screen encodes —
 * see web/blip_qr.js's render() stashing it into
 * window.BlipQR.lastRenderedText for exactly this. */
async function readRenderedText(cdp) {
  return evaluate(cdp, 'window.BlipQR.lastRenderedText');
}

/** Arm the *other* side's next scan() call to resolve with `text`
 * immediately instead of opening a camera — see web/blip_qr.js's
 * testInject. Must be set before the JOIN/SCAN button is tapped. */
async function injectNextScan(cdp, text) {
  await evaluate(cdp, `window.BlipQR.testInject = ${JSON.stringify(text)}`);
}

async function readScanError(cdp) {
  return evaluate(cdp, `(function () {
    var errors = document.querySelectorAll('.blip-hs-err');
    return errors.length ? errors[errors.length - 1].textContent || '' : '';
  })()`);
}

async function captureReportScreenshot(cdp, name) {
  await mkdir(REPORT_DIR, { recursive: true });
  const timeout = new Promise((_, reject) => setTimeout(() => reject(new Error(`timed out capturing ${name}`)), 10000));
  await Promise.race([cdp.send('Page.bringToFront'), timeout]);
  const shot = await Promise.race([
    cdp.send('Page.captureScreenshot', { format: 'png', fromSurface: false }),
    timeout,
  ]);
  await writeFile(path.join(REPORT_DIR, `${name}.png`), Buffer.from(shot.data, 'base64'));
}

test('two Chrome-for-Android tabs: QR-only Rally pairing', { skip: skipReason }, async (t) => {
  const adbBin = avd.adbPath();
  const host = await connectAndroid({ adb: adbBin, serial: HOST_SERIAL, localPort: HOST_CDP_PORT, urlIncludes: `127.0.0.1:${PORT}/rally/index.html?role=host` });
  const guest = await connectAndroid({ adb: adbBin, serial: GUEST_SERIAL, localPort: GUEST_CDP_PORT, urlIncludes: `127.0.0.1:${PORT}/rally/index.html?role=guest` });
  await host.send('Page.enable'); await host.send('Runtime.enable');
  await guest.send('Page.enable'); await guest.send('Runtime.enable');

  await Promise.all([loadRally(host), loadRally(guest)]);

  await t.test('host generates an offer and renders it as a QR code', async () => {
    await evaluate(host, "document.getElementById('net-play-btn').click()");
    await evaluate(host, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent==='HOST';}).click()");
    await waitFor(host, QR_READY, 8000);
    // guest's scan hasn't started yet (JOIN hasn't been tapped) — arm its
    // injection now, well before it actually calls scan().
    const offerText = await readRenderedText(host);
    assert.ok(offerText, 'host never rendered an offer QR with real text');
    await injectNextScan(guest, offerText);
  });

  await t.test('guest "scans" the offer (test-injected) and renders an answer QR', async () => {
    await evaluate(guest, "document.getElementById('net-play-btn').click()");
    await evaluate(guest, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent==='JOIN';}).click()");
    const outcome = await (async () => {
      for (let i = 0; i < 100; i++) {
        if (await evaluate(guest, QR_READY)) return { ok: true };
        await sleep(200);
      }
      return { ok: false, err: await readScanError(guest) || 'timed out waiting for a decode' };
    })();
    assert.ok(outcome.ok, `guest never decoded the offer QR: ${outcome.err}`);
    const answerText = await readRenderedText(guest);
    assert.ok(answerText, 'guest never rendered an answer QR with real text');
    await injectNextScan(host, answerText);
  });

  await t.test('host "scans" the answer (test-injected) and the DataChannel opens on both sides', async () => {
    await evaluate(host, "Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent.indexOf('SCAN')!==-1;}).click()");
    const hostRole = await waitFor(host, 'window.blipNetRole()', 20000);
    const guestRole = await waitFor(guest, 'window.blipNetRole()', 20000);
    assert.equal(hostRole, 1, 'host should report role 1 once its DataChannel opens');
    assert.equal(guestRole, 2, 'guest should report role 2 once its DataChannel opens');
    await captureReportScreenshot(host, '01-paired-host');
    await captureReportScreenshot(guest, '01-paired-guest');
  });

  await t.test('the match actually starts on both devices', async () => {
    // Pairing puts Rally into Serve.  The authoritative host still needs the
    // same launch gesture as a local match before it enters Play and streams
    // moving state to the guest.
    await evaluate(host, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: ' ', code: 'Space',
      }));
    `);
    await sleep(250);
    await evaluate(host, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: ' ', code: 'Space',
      }));
    `);
    const hostFrac = await waitFor(host, `(${READ_RIGHT_FRACTION}) > 0.3`, 15000);
    const guestFrac = await waitFor(guest, `(${READ_RIGHT_FRACTION}) > 0.3`, 15000);
    assert.ok(hostFrac, 'host paddle never left the Title-screen position');
    assert.ok(guestFrac, 'guest paddle never left the Title-screen position');
  });

  await t.test('guest input reaches the host and the resulting state syncs back', async () => {
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.3 && before < 0.7, `expected a centered paddle to start, got ${before}`);

    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 12000);
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }));
    `);
    await sleep(200);

    const guestAfter = await evaluate(guest, READ_RIGHT_FRACTION);
    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(guestAfter < before, `guest's own mirrored paddle should show it moved up (was ${before}, now ${guestAfter})`);
    assert.ok(hostAfter < before, `host should have actually simulated the remote input (was ${before}, now ${hostAfter})`);
    assert.ok(Math.abs(guestAfter - hostAfter) < 0.05,
      `host and guest should agree on the settled paddle position (host ${hostAfter}, guest ${guestAfter})`);
    await captureReportScreenshot(host, '02-synchronized-host');
    await captureReportScreenshot(guest, '02-synchronized-guest');
  });

  await t.test('a clean disconnect is observed on both sides', async () => {
    await evaluate(host, 'window.BlipNet.cancel()');
    const hostRole = await waitFor(host, 'window.blipNetRole() === 0', 8000);
    assert.equal(hostRole, true);
    const guestRole = await waitFor(guest, 'window.blipNetRole() === 0', 10000);
    assert.equal(guestRole, true);
  });
});
