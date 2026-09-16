// Shared plumbing for the multiplayer QR/WebRTC test suite
// (test/multiplayer.mjs). Split out so several independent `describe()`
// groups — QR-image robustness, protocol rejection of bad content, camera
// failure paths, network-blocked diagnosis, the full two-device pairing,
// and timing variants — can each launch just the headless Chromium
// instance(s) they actually need instead of every test paying for the
// full two-device dance the original single end-to-end test did.
//
// Every helper here drives the *real* code path (real `getUserMedia`,
// real jsQR decode, real RTCPeerConnection) — see test/multiplayer.mjs's
// own file header for why that distinction matters for this feature.

import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { connect, evaluate, waitFor, sleep, killAll } from './cdp.mjs';
import { chromiumBinary } from './chromium-binary.mjs';

export { evaluate, waitFor, sleep, killAll };

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const WEB_DIR = path.join(__dirname, '..', '..', 'web');
export const HTTP_PORT = 8098; // distinct from the 8080 devs run by hand

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

export function createFileServer() {
  return createServer(async (req, res) => {
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
}

// Every test in this suite that needs a debug port pulls the next one
// from here instead of hard-coding one — dozens of small, independent
// Chrome launches (one full 2-device pairing per test would be needlessly
// slow) means dozens of ports, and a shared counter is simpler and safer
// than each test file/section picking its own range and hoping they never
// collide.
// Software GL rather than no GL: Rally is a macroquad/WebGL game, and
// `--disable-gpu` leaves headless Chrome on macOS with no WebGL context
// at all, so the wasm never starts, the paddle dials never move, and
// every state-sync assertion fails on a game that was never running.
// SwiftShader renders in software, so it behaves the same on a CI box
// with no GPU as on a developer's Mac.
const HEADLESS_GL = ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'];

let nextPort = 9531;
export function allocPorts(n = 1) {
  const start = nextPort;
  nextPort += n;
  return n === 1 ? start : Array.from({ length: n }, (_, i) => start + i);
}

export function sh(cmd, args) {
  return new Promise((resolve, reject) => {
    const p = spawn(cmd, args, { stdio: 'ignore' });
    p.on('error', reject);
    p.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} exited ${code}`))));
  });
}

/** The filter chain a real "phone camera scanning a QR held a few inches
 * away" needs: the modal's dark panel color as a pad, not white paper (a
 * light quiet zone against the code, not around the whole frame — the
 * same thing writeQrVideo's own doc comment in the original test
 * explains). `scaleW`/`scaleH` independently — equal for an undistorted
 * code, unequal to simulate anisotropic stretch/skew. `extra` appends any
 * further ffmpeg video filter (blur, contrast, flip, rotate, ...). */
export function frameFilter(scaleW, scaleH, extra) {
  return `scale=${scaleW}:${scaleH}:flags=neighbor,pad=480:480:(ow-iw)/2:(oh-ih)/2:#0b1016` +
    (extra ? ',' + extra : '');
}

export async function writeQrVideo(pngPath, outPath, vf) {
  await sh('ffmpeg', ['-y', '-loop', '1', '-i', pngPath,
    '-vf', vf || frameFilter(440, 440), '-t', '8', '-pix_fmt', 'yuv420p', outPath]);
}
export async function writeBlankVideo(outPath) {
  await sh('ffmpeg', ['-y', '-f', 'lavfi', '-i', 'color=white:s=480x480:d=2', '-pix_fmt', 'yuv420p', outPath]);
}

/** A headless Chromium fed a fake camera from `camFile` (a .y4m video —
 * see writeQrVideo/writeBlankVideo). The file can be *rewritten* after
 * launch (ffmpeg overwrites it, Chrome keeps reading) — that's how a test
 * shows a QR code to an already-running "camera" mid-test rather than
 * needing to know the code's content before the browser even starts. */
export async function launchWithCamera(port, camFile) {
  const bin = chromiumBinary();
  const proc = spawn(bin, [
    '--headless=new', ...HEADLESS_GL, '--no-sandbox', '--disable-dev-shm-usage',
    '--disable-features=WebRtcHideLocalIpsWithMdns',
    '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding', '--disable-background-timer-throttling',
    '--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream',
    `--use-file-for-fake-video-capture=${camFile}`,
    `--remote-debugging-port=${port}`, '--js-flags=--max-old-space-size=192',
    'about:blank',
  ], { stdio: 'ignore' });
  const cdp = await connect(port);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  return { proc, cdp };
}

/** No fake-device flag at all — `enumerateDevices()` reports zero
 * `videoinput`s, the same as a real desktop with no webcam. Used for the
 * "no camera on this device" upfront-warning test; see
 * checkForCamera()'s own comment in web/blip_net_ui.js for why that check
 * exists at all. */
export async function launchWithNoCamera(port) {
  const bin = chromiumBinary();
  const proc = spawn(bin, [
    '--headless=new', ...HEADLESS_GL, '--no-sandbox', '--disable-dev-shm-usage',
    `--remote-debugging-port=${port}`, '--js-flags=--max-old-space-size=192',
    'about:blank',
  ], { stdio: 'ignore' });
  const cdp = await connect(port);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  return { proc, cdp };
}

export const QR_READY = `(function(){
  var c = document.querySelector('.blip-hs-panel canvas.blip-qr-canvas');
  return !!(c && c.width > 50 && c.width === c.height);
})()`;

/** The dial hand's rotation encodes the paddle fraction — see
 * shell.js's `window.blipPaddles`. See test/multiplayer.mjs's original
 * comment on why reading the live DOM (rather than intercepting a
 * callback) is the injection-free way to observe both sides. */
export const READ_RIGHT_FRACTION = `(function () {
  var el = document.getElementById('dial-hand-p2');
  if (!el) return null;
  var m = /rotate\\(([-\\d.eE]+)rad\\)/.exec(el.style.transform || '');
  if (!m) return null;
  var SWEEP = 3 * Math.PI;
  return parseFloat(m[1]) / SWEEP + 0.5;
})()`;

export async function loadRally(cdp) {
  return loadRallyAt(cdp, `http://127.0.0.1:${HTTP_PORT}`);
}

/** Same as loadRally() but against an explicit origin. A real phone
 * can't reach the harness on 127.0.0.1 — the iOS guest loads the very
 * same server over the Mac's LAN address instead. */
export async function loadRallyAt(cdp, origin) {
  await evaluate(cdp, 'true'); // ensure Runtime is ready before navigating
  await cdp.send('Page.navigate', { url: `${origin}/rally/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipNet === 'object' && typeof window.BlipQR === 'object'", 15000);
  // The coin wall: shell.js swallows every keydown while it's up — dismiss
  // it the same way a real player would, a click/tap inserting a coin.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await evaluate(cdp, "document.getElementById('need-coin-overlay').click()");
  }
}

export async function grabQrPng(cdp, outPath) {
  const b64 = await evaluate(cdp, "document.querySelector('.blip-hs-panel canvas.blip-qr-canvas').toDataURL('image/png').split(',')[1]");
  await writeFile(outPath, Buffer.from(b64, 'base64'));
}

/** Render `text` onto a scratch (off-modal) canvas with the exact same
 * window.BlipQR.render() the real HOST/JOIN screens use, save it as a
 * PNG. Used by every test that needs a QR of *specific*, controlled
 * content (a synthetic offer, garbage data, an oversized payload) without
 * driving a real RTCPeerConnection to produce it. */
export async function renderTextToPng(cdp, text, outPath) {
  await evaluate(cdp, `(function(){
    window.__harnessCanvas = document.createElement('canvas');
    document.body.appendChild(window.__harnessCanvas);
    window.BlipQR.render(window.__harnessCanvas, ${JSON.stringify(text)});
  })()`);
  const b64 = await evaluate(cdp, "window.__harnessCanvas.toDataURL('image/png').split(',')[1]");
  await writeFile(outPath, Buffer.from(b64, 'base64'));
}

/** Call window.BlipQR.scan() directly against a hidden <video>/<canvas>
 * pair, bypassing the PLAY NEARBY modal entirely — the QR-image-robustness
 * tests only care whether jsQR can decode a given camera frame, not
 * whether the surrounding pairing UI works, so this skips straight to
 * the real decode call instead of clicking through HOST/JOIN. Resolves
 * `{ ok: true, text }` on a real decode, `{ ok: false, timedOut: true }`
 * if nothing decoded inside `timeoutMs` (deliberately generous — this is
 * "did it ever decode", not a performance benchmark), or
 * `{ ok: false, errName }` if the (fake) camera itself couldn't open. */
export async function scanOnce(cdp, timeoutMs = 6000) {
  return evaluate(cdp, `
    new Promise(function (resolve) {
      var video = document.createElement('video');
      video.muted = true;
      document.body.appendChild(video);
      var overlay = document.createElement('canvas');
      overlay.width = 480; overlay.height = 480;
      document.body.appendChild(overlay);
      var settled = false, stop = null;
      var timer = setTimeout(function () {
        if (settled) return; settled = true;
        if (stop) stop();
        resolve({ ok: false, timedOut: true });
      }, ${timeoutMs});
      stop = window.BlipQR.scan(video, overlay, function (text, err) {
        if (settled) return; settled = true;
        clearTimeout(timer);
        resolve({ ok: !!text, text: text || null, errName: (err && err.name) || null });
      });
    })
  `);
}

/** A realistic-shaped (fingerprint, ice-ufrag/pwd, setup, sctp) but fully
 * synthetic and deterministic offer SDP — same fields slimSdpForQr()
 * leaves alone, `candidateCount` host candidates in the format
 * blip_sdp_slim.js's parseCandidateLine() expects. Deterministic (unlike
 * a real getUserMedia-free `RTCPeerConnection.createOffer()`, whose
 * candidate set depends on the machine's own network interfaces) so the
 * QR-distortion pass/fail thresholds this suite asserts don't shift
 * between machines or CI runners. */
export function makeSyntheticOfferSdp(candidateCount = 4) {
  const lines = [
    'v=0', 'o=- 1234567890 2 IN IP4 127.0.0.1', 's=-', 't=0 0',
    'a=group:BUNDLE 0', 'm=application 9 UDP/DTLS/SCTP webrtc-datachannel',
    'c=IN IP4 0.0.0.0', 'a=ice-ufrag:abcd', 'a=ice-pwd:0123456789abcdef0123456789ab',
    'a=fingerprint:sha-256 AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99:AA:BB:CC:DD:EE:FF:00:11:22:33:44:55:66:77:88:99',
    'a=setup:actpass', 'a=mid:0', 'a=sctp-port:5000',
  ];
  for (let i = 0; i < candidateCount; i++) {
    lines.push(`a=candidate:${i} 1 udp 2122260223 192.168.1.${10 + i} ${50000 + i} typ host generation 0 ufrag abcd network-cost 999`);
  }
  return lines.join('\r\n') + '\r\n';
}

/** Rewrite every UDP host candidate's address to a non-routable one
 * (10.255.255.1 — a real reserved-but-unroutable test address, not a
 * made-up string) while leaving the rest of the SDP untouched. This is
 * the exact technique docs/multiplayer.md's Testing section describes
 * for reproducing "device-to-device traffic is blocked" (WiFi client/AP
 * isolation, a VPN routing local traffic through a remote tunnel, an
 * OS firewall) without needing real firewall rules or two separate
 * networks: the candidates are real enough to start ICE checks, just
 * unreachable, which is exactly what "something between the two devices
 * is dropping this traffic" looks like from getStats(). */
export function blockAllCandidates(sdp) {
  return sdp.split(/\r\n/).map((line) => {
    if (line.indexOf('a=candidate:') !== 0) return line;
    const parts = line.split(' ');
    if ((parts[2] || '').toLowerCase() === 'udp') parts[4] = '10.255.255.1';
    return parts.join(' ');
  }).join('\r\n');
}

export async function clickHsBtn(cdp, label) {
  await evaluate(cdp, `(function(){
    var b = Array.from(document.querySelectorAll('.blip-hs-btn')).find(function (b) {
      return b.textContent.indexOf(${JSON.stringify(label)}) !== -1;
    });
    if (!b) throw new Error('no .blip-hs-btn matching ' + ${JSON.stringify(label)});
    b.click();
  })()`);
}

export async function openModal(cdp) {
  await evaluate(cdp, "document.getElementById('net-play-btn').click()");
}

export async function isAtChoiceScreen(cdp) {
  return evaluate(cdp, "!!Array.from(document.querySelectorAll('.blip-hs-btn')).find(function(b){return b.textContent==='HOST';})");
}

export async function getStatusText(cdp) {
  return evaluate(cdp, "document.querySelector('.blip-net-status-text') ? document.querySelector('.blip-net-status-text').textContent : null");
}

/** The join-scan step's own error line — distinct from the modal-wide
 * camera warning banner (openModal()'s cameraWarn), which shares the same
 * `.blip-hs-err` class but is a different element; both exist on screen
 * at once once JOIN has been tapped, so this always wants the last one. */
export async function getScanErrText(cdp) {
  return evaluate(cdp, "(Array.from(document.querySelectorAll('.blip-hs-err')).pop() || {}).textContent || ''");
}

export async function getCameraWarnText(cdp) {
  return evaluate(cdp, "(document.querySelector('.blip-hs-modal > .blip-hs-panel > .blip-hs-err') || {}).textContent || ''");
}

/** Poll a local async predicate (as opposed to `waitFor()`, which polls a
 * JS expression evaluated *in the page*) until it returns a truthy value
 * or `timeoutMs` elapses. For checks that read the DOM through one of
 * this module's own helper functions rather than a raw CDP expression
 * string. */
export async function pollUntil(fn, timeoutMs = 5000, intervalMs = 150) {
  const start = Date.now();
  for (;;) {
    const v = await fn();
    if (v) return v;
    if (Date.now() - start > timeoutMs) throw new Error('timed out waiting for condition');
    await sleep(intervalMs);
  }
}

export async function getTermLines(cdp) {
  return evaluate(cdp, "Array.from(document.querySelectorAll('.blip-net-term-line')).map(function(e){return e.textContent;})");
}
