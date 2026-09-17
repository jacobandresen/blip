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
import { launchEngine } from './engine.mjs';
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

/** Start a server and resolve once it is actually listening.
 *
 * The obvious `new Promise((r) => server.listen(port, r))` has no error
 * path: if the port is taken, 'error' fires, the callback never runs,
 * and the promise never settles. Under `node --test` that surfaces as a
 * suite producing *no output at all* and hanging until something kills
 * it — the worst possible failure mode, and one that looks like a
 * device or network problem rather than a stray process holding a port.
 * (It was a stray process holding a port.) */
export function listenOn(server, port) {
  return new Promise((resolve, reject) => {
    const onError = (err) => {
      server.removeListener('listening', onListening);
      reject(new Error(`could not listen on port ${port}: ${err.code || err.message}` +
        (err.code === 'EADDRINUSE' ? ' — another server (or a previous test run) is holding it' : '')));
    };
    const onListening = () => {
      server.removeListener('error', onError);
      resolve(server);
    };
    server.once('error', onError);
    server.once('listening', onListening);
    server.listen(port);
  });
}

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
/** A file server plus one browser, both torn down automatically.
 *
 * Every browser-driven suite opened with the same eight lines: make a
 * server, listen, launch, and register a t.after that closes both. That
 * is where the EADDRINUSE hang came from — one of those copies wrapped
 * listen() in a promise with no error path, and a stray process holding
 * the port turned into a suite that produced no output and never
 * finished. Written once, that class of bug has one place to live. */
export async function openPage(t, engine = 'chromium', opts = {}) {
  const server = createFileServer();
  await listenOn(server, HTTP_PORT);
  const handle = await launchEngine(engine, opts);
  t.after(async () => {
    await handle.browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  return { ...handle, server };
}

/** Two browsers with Rally already loaded on both — the shape almost
 * every multiplayer test needs before it can do anything interesting. */
export async function openPair(t, hostEngine = 'chromium', guestEngine = 'chromium') {
  const server = createFileServer();
  await listenOn(server, HTTP_PORT);
  const hostBrowser = await launchEngine(hostEngine);
  const guestBrowser = await launchEngine(guestEngine);
  t.after(async () => {
    await hostBrowser.browser.close().catch(() => {});
    await guestBrowser.browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  await Promise.all([loadRally(hostBrowser.cdp), loadRally(guestBrowser.cdp)]);
  return { host: hostBrowser.cdp, guest: guestBrowser.cdp, hostBrowser, guestBrowser, server };
}

export function rewriteRoutes(payload, address) {
  // Handles both payload shapes, because the pairing QR carries the
  // compact form (`B1|ufrag|pwd|fp|setup|host:port,...`, see packForQr in
  // web/blip_sdp_slim.js) while the SDP path still exists as a fallback.
  // A rewriter that knew only about `a=candidate:` lines would quietly
  // do nothing to a compact payload — which is not a failing test but a
  // *passing* one that has stopped testing anything, since the
  // connection then succeeds and every "blocked" assertion inverts.
  if (payload.startsWith('B1|')) {
    const parts = payload.split('|');
    parts[5] = (parts[5] || '').split(',').filter(Boolean).map((route) => {
      const at = route.lastIndexOf(':');
      return `${address}:${route.slice(at + 1)}`;
    }).join(',');
    return parts.join('|');
  }
  return payload.split(/\r\n/).map((line) => {
    if (line.indexOf('a=candidate:') !== 0) return line;
    const parts = line.split(' ');
    if ((parts[2] || '').toLowerCase() === 'udp') parts[4] = address;
    return parts.join(' ');
  }).join('\r\n');
}

export function blockAllCandidates(payload) {
  return rewriteRoutes(payload, '10.255.255.1');
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
