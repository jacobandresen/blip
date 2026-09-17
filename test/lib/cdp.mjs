// Minimal Chrome DevTools Protocol client over a raw WebSocket — no
// puppeteer/playwright dependency, just Node's own `net`/`http`/`crypto`.
// Used by test/multiplayer-pairing.mjs (and any future headless browser test) to
// drive real Chromium instances directly: navigate, run JS via
// Runtime.evaluate, dispatch synthetic input, and get real page/console
// errors back, all over the same protocol DevTools itself uses.
//
// This existed only as a series of one-off scratch scripts across several
// playtest sessions before now; committing it is deliberate — the
// two-headless-Chromium multiplayer test needs it to keep working as the
// feature evolves, not just once by hand. See docs/multiplayer.md's
// Testing section.

import http from 'node:http';
import net from 'node:net';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import { chromiumBinary } from './chromium-binary.mjs';

export const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function httpJSON(port, path) {
  return new Promise((resolve, reject) => {
    const req = http.get({ host: '127.0.0.1', port, path }, (r) => {
      let body = '';
      r.on('data', (d) => { body += d; });
      r.on('end', () => {
        try { resolve(JSON.parse(body)); } catch (e) { reject(e); }
      });
    });
    req.setTimeout(3000, () => req.destroy(new Error(`timed out reading CDP http://${port}${path}`)));
    req.on('error', reject);
  });
}

/** Connect to a Chromium instance already listening on `--remote-debugging-port=port`
 * (retries for a few seconds — the process needs a moment to open the port).
 *
 * `matchUrl`, if given, picks the *most recently opened* page target whose
 * `url` contains that substring instead of just the first page target —
 * needed once there's more than one tab open. Chrome-for-Android in
 * particular tends to accumulate one tab per `am start
 * android.intent.action.VIEW` call rather than reusing the existing one;
 * older matching tabs can be backgrounded/frozen (and backgrounded tabs
 * get camera-permission requests auto-denied), so the newest match is the
 * one actually in the foreground. See `connectAndroid` below. */
export async function connect(port, matchUrl, { commandTimeoutMs = 10000 } = {}) {
  let list;
  for (let i = 0; i < 50; i++) {
    try {
      list = await httpJSON(port, '/json/list');
      if (matchUrl && !list.some((t) => t.type === 'page' && t.url?.includes(matchUrl))) {
        list = null; // keep polling until the matching tab shows up
      }
      if (list) break;
    } catch { /* keep polling */ }
    await sleep(200);
  }
  if (!list) throw new Error(`chromium never opened its debugger port ${port}` + (matchUrl ? ` (waiting for tab matching "${matchUrl}")` : ''));
  const matches = matchUrl
    ? list.filter((t) => t.type === 'page' && t.url?.includes(matchUrl) && t.webSocketDebuggerUrl)
    : list.filter((t) => t.type === 'page' && t.webSocketDebuggerUrl);
  if (!matches.length) throw new Error(`no matching page target on port ${port}`);
  // `/json/list`'s order isn't reliably chronological, but target ids are
  // assigned incrementally as tabs are created. Chromium uses hexadecimal-
  // looking ids; use BigInt so long ids are compared without precision loss.
  // If a browser uses a non-hex id, retain the endpoint's ordering rather
  // than accidentally treating every candidate as NaN.
  function targetRank(id) {
    const hex = String(id || '').replace(/^page_/, '').replace(/-/g, '');
    return /^[0-9a-f]+$/i.test(hex) ? BigInt(`0x${hex}`) : null;
  }
  const page = matches.reduce((a, b) => {
    const aId = targetRank(a.id);
    const bId = targetRank(b.id);
    return aId !== null && bId !== null && bId > aId ? b : a;
  });
  const u = new URL(page.webSocketDebuggerUrl);

  return new Promise((resolve, reject) => {
    const key = crypto.randomBytes(16).toString('base64');
    const sock = net.connect(u.port, u.hostname, () => {
      sock.write(
        `GET ${u.pathname}${u.search} HTTP/1.1\r\nHost: ${u.host}\r\n` +
        `Upgrade: websocket\r\nConnection: Upgrade\r\n` +
        `Sec-WebSocket-Key: ${key}\r\nSec-WebSocket-Version: 13\r\n\r\n`
      );
    });
    sock.setTimeout(10000, () => {
      sock.destroy(new Error(`timed out connecting to CDP target ${u.pathname}`));
    });
    let buf = Buffer.alloc(0);
    let shook = false;
    let id = 0;
    const waiters = new Map();
    const listeners = [];

    const api = {
      send(method, params) {
        const mid = ++id;
        const payload = Buffer.from(JSON.stringify({ id: mid, method, params: params || {} }));
        const n = payload.length;
        let hdr;
        if (n < 126) hdr = Buffer.from([0x81, 0x80 | n]);
        else if (n < 65536) hdr = Buffer.concat([Buffer.from([0x81, 0x80 | 126]), Buffer.from([n >> 8, n & 255])]);
        else {
          const b = Buffer.alloc(8);
          b.writeBigUInt64BE(BigInt(n));
          hdr = Buffer.concat([Buffer.from([0x81, 0x80 | 127]), b]);
        }
        const mask = crypto.randomBytes(4);
        const masked = Buffer.alloc(n);
        for (let i = 0; i < n; i++) masked[i] = payload[i] ^ mask[i % 4];
        sock.write(Buffer.concat([hdr, mask, masked]));
        return new Promise((resolveCall, rejectCall) => {
          const timer = setTimeout(() => {
            waiters.delete(mid);
            rejectCall(new Error(`timed out waiting for CDP command ${method}`));
          }, commandTimeoutMs);
          waiters.set(mid, { resolveCall, rejectCall, timer });
        });
      },
      /** `fn(method, params)` fires for every CDP event (not call replies). */
      on(fn) { listeners.push(fn); },
      close() { try { sock.end(); } catch {} },
    };

    sock.on('data', (chunk) => {
      buf = Buffer.concat([buf, chunk]);
      if (!shook) {
        const i = buf.indexOf('\r\n\r\n');
        if (i < 0) return;
        buf = buf.subarray(i + 4);
        shook = true;
        // The socket timeout only protects the HTTP/WebSocket handshake.
        // Leaving it armed kills an otherwise healthy Android target while
        // the other emulator is being initialized.
        sock.setTimeout(0);
        resolve(api);
      }
      while (buf.length >= 2) {
        const len0 = buf[1] & 127;
        let off = 2;
        let len = len0;
        if (len0 === 126) { len = buf.readUInt16BE(2); off = 4; }
        else if (len0 === 127) { len = Number(buf.readBigUInt64BE(2)); off = 10; }
        if (buf.length < off + len) break;
        const data = buf.subarray(off, off + len);
        buf = buf.subarray(off + len);
        let msg;
        try { msg = JSON.parse(data.toString()); } catch { continue; }
        if (msg.id && waiters.has(msg.id)) {
          const w = waiters.get(msg.id);
          waiters.delete(msg.id);
          clearTimeout(w.timer);
          if (msg.error) w.rejectCall(new Error(msg.error.message + ' :: ' + JSON.stringify(msg.error.data || '')));
          else w.resolveCall(msg.result);
        } else if (msg.method) {
          listeners.forEach((fn) => { try { fn(msg.method, msg.params); } catch {} });
        }
      }
    });
    sock.on('error', reject);
  });
}

/**
 * Launch a headless Chromium listening on `port` and return `{ proc, cdp }`.
 * Flags match what this repo's own headless playtests have needed in
 * practice: sandboxing off (containers), shared-memory off (small
 * `/dev/shm`), and a capped heap so two instances at once (the
 * multiplayer test's whole point) don't starve the host.
 */
export async function launch(port, extraArgs = []) {
  const bin = chromiumBinary();
  const proc = spawn(bin, [
    '--headless=new',
    // Software WebGL, not none — see HEADLESS_GL in
    // test/lib/multiplayer-harness.mjs for why a GPU-less headless
    // Chrome silently breaks every wasm-game assertion.
    '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
    '--no-sandbox',
    '--disable-dev-shm-usage',
    // Chrome hides local ICE candidates behind a per-instance `.local`
    // mDNS hostname by default (a privacy feature) — resolving that
    // between two *separate* headless instances on one host is exactly
    // the kind of thing a sandboxed/CI network namespace can't be
    // trusted to do reliably. Use real IPs for candidates instead; see
    // docs/multiplayer.md's Testing section.
    '--disable-features=WebRtcHideLocalIpsWithMdns',
    // A headless page is never "foreground" — Chrome's background-tab
    // throttling (meant for an unfocused real tab) can otherwise kick in
    // almost immediately and has been observed tearing down an
    // already-open RTCDataChannel within a few hundred ms of connecting.
    '--disable-backgrounding-occluded-windows',
    '--disable-renderer-backgrounding',
    '--disable-background-timer-throttling',
    `--remote-debugging-port=${port}`,
    '--js-flags=--max-old-space-size=192',
    ...extraArgs,
    'about:blank',
  ], { stdio: 'ignore' });
  const cdp = await connect(port);
  await cdp.send('Page.enable');
  await cdp.send('Runtime.enable');
  return { proc, cdp };
}

export function killAll(procs) {
  for (const p of procs) { try { p.kill('SIGKILL'); } catch {} }
}

/**
 * Attach CDP to a page already open in Chrome-for-Android, no extra Chrome
 * flags needed (remote debugging is on by default; it's exposed over an
 * abstract Unix domain socket rather than a TCP port). `adb forward` bridges
 * that socket to a local TCP port on the host, which then speaks the exact
 * same `/json/list` + WebSocket protocol as desktop Chromium — see
 * docs/android-multiplayer-test-plan.md.
 *
 * `urlIncludes` is required here (unlike `connect`) because a real Chrome
 * instance always has other tabs (at minimum its New Tab Page).
 */
export async function connectAndroid({ adb, serial, localPort, urlIncludes }) {
  await new Promise((resolve, reject) => {
    const p = spawn(adb, ['-s', serial, 'forward', `tcp:${localPort}`, 'localabstract:chrome_devtools_remote'], { stdio: 'ignore' });
    p.on('exit', (code) => (code === 0 ? resolve() : reject(new Error(`adb forward exited ${code}`))));
    p.on('error', reject);
  });
  return connect(localPort, urlIncludes, { commandTimeoutMs: 30000 });
}

/** `Runtime.evaluate` with `returnByValue` + `awaitPromise`, throwing on a
 * JS-side exception instead of returning a silent `undefined`. */
export async function evaluate(cdp, expression) {
  const r = await cdp.send('Runtime.evaluate', {
    expression, returnByValue: true, awaitPromise: true,
  });
  if (r.exceptionDetails) {
    throw new Error(r.exceptionDetails.exception?.description || JSON.stringify(r.exceptionDetails));
  }
  return r.result.value;
}

/** Poll `evaluate(cdp, expression)` until it's truthy or `timeoutMs` elapses. */
export async function waitFor(cdp, expression, timeoutMs = 15000, intervalMs = 100) {
  const start = Date.now();
  for (;;) {
    const v = await evaluate(cdp, expression);
    if (v) return v;
    if (Date.now() - start > timeoutMs) {
      throw new Error(`timed out waiting for truthy: ${expression}`);
    }
    await sleep(intervalMs);
  }
}
