// A CDP-shaped adapter over macOS's built-in `safaridriver` (W3C
// WebDriver), so desktop Safari can be driven by the same helpers in
// test/lib/multiplayer-harness.mjs that drive Chromium over CDP.
//
// Why a shim instead of a second set of helpers: every helper in the
// harness reaches the page through cdp.mjs's `evaluate()`, which only
// ever calls `cdp.send('Runtime.evaluate', { expression })` and reads
// back `{ result: { value } }` / `{ exceptionDetails }`. Implementing
// exactly those two messages (plus `Page.navigate`) against WebDriver's
// /execute/async and /url endpoints makes a Safari session a drop-in for
// a CDP connection — `loadRally`, `clickHsBtn`, `waitFor`, all of it,
// unchanged.
//
// Safari matters here because it is the *only* engine the guest side can
// run on iOS, and until this file existed the multiplayer suite ran on
// Chromium alone — WebKit's RTCPeerConnection was never exercised.

import { spawn } from 'node:child_process';
import { sleep } from './cdp.mjs';

const SAFARIDRIVER = '/usr/bin/safaridriver';

async function wd(port, method, path, body) {
  const res = await fetch(`http://127.0.0.1:${port}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await res.text();
  let json;
  try { json = JSON.parse(text); } catch { throw new Error(`safaridriver returned non-JSON (${res.status}): ${text.slice(0, 300)}`); }
  if (json.value && json.value.error) {
    throw new Error(`webdriver ${json.value.error}: ${json.value.message || ''}`);
  }
  return json.value;
}

/** Start safaridriver and wait for /status to report ready. */
export async function startSafariDriver(port) {
  const proc = spawn(SAFARIDRIVER, ['-p', String(port)], { stdio: 'ignore' });
  for (let i = 0; i < 50; i++) {
    try {
      const v = await wd(port, 'GET', '/status');
      if (v && v.ready) return proc;
    } catch { /* not up yet */ }
    await sleep(200);
  }
  proc.kill();
  throw new Error(
    `safaridriver never became ready on port ${port}. ` +
    'Remote automation must be enabled once, by hand: `sudo safaridriver --enable`, ' +
    "plus Safari > Settings > Advanced > Show features for web developers, then Develop > Allow Remote Automation.");
}

/** Open a Safari automation session and return a CDP-shaped handle. */
export async function connectSafari(port) {
  const v = await wd(port, 'POST', '/session', {
    capabilities: { alwaysMatch: { browserName: 'safari' } },
  });
  const sessionId = v.sessionId;

  // Promise-aware on purpose: cdp.mjs's evaluate() passes awaitPromise,
  // and several harness expressions (getStats, BlipQR.scan) do resolve a
  // promise rather than a plain value.
  const EVAL = `
    var done = arguments[arguments.length - 1];
    try {
      Promise.resolve(eval(arguments[0])).then(
        function (v) { done({ ok: true, v: v }); },
        function (e) { done({ ok: false, e: String((e && e.stack) || e) }); });
    } catch (e) { done({ ok: false, e: String((e && e.stack) || e) }); }
  `;

  return {
    kind: 'safari',
    sessionId,
    async send(method, params = {}) {
      if (method === 'Runtime.evaluate') {
        const r = await wd(port, 'POST', `/session/${sessionId}/execute/async`, {
          script: EVAL, args: [params.expression],
        });
        if (r && r.ok) return { result: { value: r.v } };
        return { exceptionDetails: { exception: { description: (r && r.e) || 'unknown page error' } } };
      }
      if (method === 'Page.navigate') {
        await wd(port, 'POST', `/session/${sessionId}/url`, { url: params.url });
        return {};
      }
      // Runtime.enable / Page.enable are CDP bookkeeping with no
      // WebDriver equivalent and nothing to do — accept them so shared
      // setup code doesn't have to branch on the driver.
      if (method === 'Runtime.enable' || method === 'Page.enable') return {};
      throw new Error(`safari-webdriver shim does not implement ${method}`);
    },
    async close() {
      try { await wd(port, 'DELETE', `/session/${sessionId}`); } catch { /* already gone */ }
    },
  };
}
