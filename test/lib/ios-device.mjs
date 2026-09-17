// Talking to a real, USB-attached iPhone from the test suite.
//
// iOS has no adb and no CDP. Its equivalent is the WebKit remote
// inspector, and since iOS 17 that lives behind Apple's RemoteXPC
// tunnel — which is why `ios_webkit_debug_proxy`, the tool that used to
// relay it, no longer connects (it fails during the TLS handshake and
// reports no targets at all). pymobiledevice3 does speak the current
// protocol, and its `webinspector cdp` subcommand re-exposes Safari's
// inspector as an ordinary CDP endpoint on localhost — so once it is
// running, test/lib/cdp.mjs drives an iPhone with exactly the same
// `connect()` / `evaluate()` it uses for desktop Chrome.
//
// What still has to be done by hand, once, on the phone:
//   Settings > Apps > Safari > Advanced > Web Inspector       -> on
//   Settings > Apps > Safari > Advanced > Remote Automation   -> on
// and the device must be unlocked and trusted for this Mac.

import { spawn } from 'node:child_process';
import { networkInterfaces } from 'node:os';
import { connect, evaluate, sleep } from './cdp.mjs';

export const INSPECTOR_PORT = 9222;

/** The Mac's LAN address. The phone loads the harness's web server over
 * WiFi, so 127.0.0.1 is useless to it. */
export function lanAddress() {
  for (const addrs of Object.values(networkInterfaces())) {
    for (const a of addrs || []) {
      if (a.family === 'IPv4' && !a.internal) return a.address;
    }
  }
  throw new Error('no non-loopback IPv4 address — the phone cannot reach this machine');
}

async function listTargets(port) {
  const res = await fetch(`http://127.0.0.1:${port}/json`, { signal: AbortSignal.timeout(5000) });
  return res.json();
}

/** Start `pymobiledevice3 webinspector cdp` unless something is already
 * serving that port. Returns the child process, or null if the bridge
 * was already up (in which case the caller must not kill it). */
export async function startInspectorBridge(port = INSPECTOR_PORT) {
  try {
    await listTargets(port);
    return null; // already running
  } catch { /* not up yet */ }

  const proc = spawn('pymobiledevice3', ['webinspector', 'cdp'], {
    stdio: 'ignore',
    env: { ...process.env, PATH: `${process.env.HOME}/.local/bin:${process.env.PATH}` },
  });
  for (let i = 0; i < 40; i++) {
    await sleep(500);
    try { await listTargets(port); return proc; } catch { /* still starting */ }
  }
  proc.kill();
  throw new Error('pymobiledevice3 webinspector cdp never started serving — is it installed? (uv tool install pymobiledevice3)');
}


/** A sleeping phone does not refuse connections — it accepts them and
 * then never answers, so every step downstream inherits an open-ended
 * hang (the bridge itself waits ~60s before giving up with "device did
 * not report an inspection target in time"). Every device call the
 * harness makes during preflight goes through here so the failure is a
 * fast, readable one instead of a stalled test run. */
export async function withDeadline(promise, ms, message) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(message)), ms); }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

/** Wait for Safari on the phone to expose an inspectable page. */
export async function waitForSafariTarget(port = INSPECTOR_PORT, timeoutMs = 30000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    let targets = [];
    try { targets = await listTargets(port); } catch { /* bridge hiccup */ }
    const page = targets.find((t) => t.type === 'page' || t.webSocketDebuggerUrl);
    if (page) return page;
    if (Date.now() > deadline) {
      throw new Error(
        'no inspectable Safari page on the device. Unlock the phone, open Safari to any page, ' +
        'and confirm Settings > Apps > Safari > Advanced > Web Inspector is on.');
    }
    await sleep(1000);
  }
}

/** A locked/sleeping phone keeps *listing* its target long after it has
 * stopped answering on it, so presence in /json proves nothing. The only
 * reliable check is to evaluate something and see whether a reply comes
 * back before the timeout. */
export async function requireAwakeDevice(cdp) {
  try {
    await withDeadline(evaluate(cdp, '1 + 1'), 20000, 'no reply within 20s');
  } catch (e) {
    throw new Error(
      `the iPhone is listed but not answering (${e.message}). Its screen is almost certainly asleep — ` +
      'wake and unlock it. The harness takes a Screen Wake Lock as its first step so it stays awake ' +
      'for the rest of the run; see test/lib/ios-wakelock.mjs.');
  }
}

/** Is there a phone here that will actually answer?
 *
 * Listing a target proves nothing: a sleeping device keeps advertising
 * its pages long after it has stopped responding on them, and every
 * call then accepts and never completes. Worse, the half-open socket
 * keeps node alive, so even a per-call deadline leaves the process
 * hanging after the tests themselves have given up.
 *
 * So the question is asked once, cheaply, with its own socket that is
 * closed either way, before any server or browser is started. A phone
 * that cannot answer a `1+1` is a phone this suite should skip, not
 * one it should spend two minutes timing out against. */
export async function deviceAnswers(port = INSPECTOR_PORT, timeoutMs = 8000) {
  let cdp = null;
  try {
    const targets = await listTargets(port);
    if (!targets.length) return false;
    cdp = await withDeadline(connect(port, undefined, { commandTimeoutMs: timeoutMs }), timeoutMs, 'attach');
    await withDeadline(evaluate(cdp, '1 + 1'), timeoutMs, 'probe');
    return true;
  } catch {
    return false;
  } finally {
    try { if (cdp && cdp.close) cdp.close(); } catch { /* best effort */ }
  }
}

export async function connectDevice(port = INSPECTOR_PORT) {
  await waitForSafariTarget(port);
  // Generous per-command: every round trip crosses USB and the
  // RemoteXPC tunnel. Bounded overall, because attaching to a sleeping
  // phone hangs rather than fails.
  const cdp = await withDeadline(
    connect(port, undefined, { commandTimeoutMs: 30000 }),
    45000,
    'timed out attaching to the iPhone — its screen is almost certainly asleep; wake and unlock it');
  await withDeadline(cdp.send('Runtime.enable'), 20000, 'iPhone did not enable Runtime — asleep?');
  await withDeadline(cdp.send('Page.enable'), 20000, 'iPhone did not enable Page — asleep?');
  return cdp;
}
