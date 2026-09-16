// Two-device Rally against a *real, physical iPhone*.
//
// The Mac hosts the match in WebKit and the phone joins it over WiFi,
// signalled by the same QR payload a camera would have decoded. This is
// the one test in the suite where the guest is real hardware: a real
// iOS Safari, a real WebRTC stack, real WiFi between the two peers,
// nothing simulated but the photograph of the QR code.
//
// Setup (see test/lib/ios-device.mjs for the details and the why):
//   - iPhone attached over USB and trusted by this Mac.
//   - Settings > Apps > Safari > Advanced > Web Inspector      -> on
//   - Settings > Apps > Safari > Advanced > Remote Automation   -> on
//   - `uv tool install pymobiledevice3` (the bridge is auto-started).
//   - Phone unlocked, with Safari open. The run tries to take a Screen
//     Wake Lock first, so a managed device that cannot have Auto-Lock
//     set to Never stays awake for the rest of the test. Over plain HTTP
//     that is unavailable (navigator.wakeLock is secure-context-only),
//     and the run reports so rather than failing — but the phone can
//     then re-lock mid-test, which surfaces as a timeout.
//
// Note what this test does and does not show. It injects the scanned QR
// payload, so it exercises the real SDP, the real WebRTC stack and real
// WiFi between the peers, but not the camera. Over plain HTTP it cannot:
// iOS exposes no navigator.mediaDevices outside a secure context, so a
// human could not complete PLAY NEARBY on this same server. See
// docs/multiplayer.md, "iOS needs HTTPS for the camera".
//
// The host engine is configurable. It defaults to Playwright's WebKit —
// the same engine as Safari, and the one pairing that needs no
// machine-level setup. BLIP_IOS_HOST=safari drives the real Safari.app
// through safaridriver instead, which additionally requires
// `sudo safaridriver --enable` and Develop > Allow Remote Automation.
//
// Skipped, not failed, when no phone is attached: this is a hardware
// test, and a laptop with nothing plugged into it is not a broken build.

import test from 'node:test';
import assert from 'node:assert/strict';
import { launchEngine } from './lib/engine.mjs';
import { startSafariDriver, connectSafari } from './lib/safari-webdriver.mjs';
import { pairOverQr } from './lib/pairing.mjs';
import { acquireWakeLock, wakeLockStatus } from './lib/ios-wakelock.mjs';
import {
  startInspectorBridge, connectDevice, requireAwakeDevice, lanAddress, INSPECTOR_PORT,
} from './lib/ios-device.mjs';
import {
  createFileServer, HTTP_PORT, loadRally, loadRallyAt, READ_RIGHT_FRACTION,
  evaluate, waitFor, sleep,
} from './lib/multiplayer-harness.mjs';

const HOST_ENGINE = process.env.BLIP_IOS_HOST || 'webkit';

async function deviceAvailable() {
  try {
    const res = await fetch(`http://127.0.0.1:${INSPECTOR_PORT}/json`, { signal: AbortSignal.timeout(3000) });
    return (await res.json()).length > 0;
  } catch { return false; }
}

test('two-device Rally: Mac hosts, a real iPhone joins over WiFi', async (t) => {
  let bridge = null;
  try {
    bridge = await startInspectorBridge();
  } catch (e) {
    t.skip(`no iOS inspector bridge: ${e.message}`);
    return;
  }
  if (!await deviceAvailable()) {
    // Kill it here rather than leaning on t.after, which is not
    // registered until further down: skipping must not leak the bridge
    // process this test just started.
    if (bridge) bridge.kill();
    t.skip('no inspectable Safari page on an attached iPhone — see this file\'s header for setup');
    return;
  }

  const server = createFileServer();
  await new Promise((r) => server.listen(HTTP_PORT, r));
  const origin = `http://${lanAddress()}:${HTTP_PORT}`;

  const guest = await connectDevice();
  await requireAwakeDevice(guest);

  let hostHandle, host, safariProc;
  if (HOST_ENGINE === 'safari') {
    safariProc = await startSafariDriver(4446);
    host = await connectSafari(4446);
  } else {
    hostHandle = await launchEngine(HOST_ENGINE);
    host = hostHandle.cdp;
  }

  t.after(async () => {
    if (hostHandle) await hostHandle.browser.close().catch(() => {});
    if (host && host.close) await host.close().catch(() => {});
    if (safariProc) safariProc.kill();
    if (bridge) bridge.kill();
    await new Promise((r) => server.close(r));
  });

  await t.test('the phone is asked to stay awake for the rest of the run', async () => {
    // First, before anything slow: a managed iPhone re-locks on the
    // profile's timer, and a locked phone stops answering the inspector
    // mid-test in a way that looks like an unrelated timeout.
    await loadRallyAt(guest, origin);
    const got = await acquireWakeLock(guest);
    t.diagnostic(`wake lock: ${got} ${JSON.stringify(await wakeLockStatus(guest))}`);

    // Not an assertion, because the wake lock is simply unavailable in
    // the setup this test runs in: the phone loads the harness over
    // plain HTTP at a LAN address, which is not a secure context, and
    // navigator.wakeLock does not exist outside one. Failing the run
    // over it would report a broken product for an untrusted-origin
    // limitation of the test rig. The consequence is real though — the
    // phone can re-lock mid-run — so it is said out loud.
    if (got !== 'acquired') {
      const secure = await evaluate(guest, 'isSecureContext');
      t.diagnostic(`no wake lock (secure context: ${secure}) — the phone may re-lock mid-run ` +
        'and later steps would then fail as timeouts. Serve the harness over HTTPS to hold the screen on.');
    }
  });

  await t.test('Rally is really running on the phone, in Safari', async () => {
    const ua = await evaluate(guest, 'navigator.userAgent');
    assert.match(ua, /iPhone/, `expected to be driving an iPhone, got: ${ua}`);
    assert.equal(await evaluate(guest, 'typeof RTCPeerConnection'), 'function');
    assert.ok(await evaluate(guest, "!!document.getElementById('glcanvas')"), 'game canvas missing');
  });

  let paired;

  await t.test('the QR exchange completes and the DataChannel opens on both sides', async () => {
    await loadRally(host);
    paired = await pairOverQr(host, guest, { connectTimeoutMs: 45000 });
    assert.equal(paired.hostRole, 1, 'Mac should report role 1 (host)');
    assert.equal(paired.guestRole, 2, 'iPhone should report role 2 (guest)');
  });

  await t.test('the two peers connected directly over the LAN', async () => {
    // No ICE servers are configured at all, so a connection here is
    // necessarily peer-to-peer across the WiFi — nothing is relayed.
    const dbg = await evaluate(guest, 'window.__blipNetDebug()');
    assert.equal(dbg.dcState, 'open', `phone DataChannel is ${dbg.dcState}`);
    assert.ok(['connected', 'completed'].includes(dbg.pcIceConnectionState),
      `phone ICE state is ${dbg.pcIceConnectionState}`);
    assert.match(paired.answer, /typ host/, 'the phone offered no host candidate');
  });

  await t.test('the match starts on both devices', async () => {
    assert.ok(await waitFor(host, `(${READ_RIGHT_FRACTION}) > 0.3`, 20000), 'Mac never left the title screen');
    assert.ok(await waitFor(guest, `(${READ_RIGHT_FRACTION}) > 0.3`, 20000), 'phone never left the title screen');
  });

  await t.test('input on the phone moves the paddle the Mac is simulating', async () => {
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.3 && before < 0.7, `expected a centered paddle, got ${before}`);

    const press = (type) => evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('${type}', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }))`);

    // The phone renders only what the Mac simulates and sends back, so
    // the phone's own view moving is proof of a full WiFi round trip.
    // Released as soon as that lands — see test/multiplayer.mjs on why a
    // mid-motion comparison between the two sides is flaky by design.
    await press('keydown');
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 15000);
    await press('keyup');
    await sleep(300); // packets still in flight over WiFi

    const guestAfter = await evaluate(guest, READ_RIGHT_FRACTION);
    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(guestAfter < before, `phone's mirrored paddle should have moved up (${before} -> ${guestAfter})`);
    assert.ok(hostAfter < before, `Mac should have simulated the phone's input (${before} -> ${hostAfter})`);
    assert.ok(Math.abs(guestAfter - hostAfter) < 0.08,
      `the two devices disagree on the paddle (Mac ${hostAfter}, phone ${guestAfter})`);
  });

  await t.test('a clean disconnect is observed on both sides', async () => {
    await evaluate(host, 'window.BlipNet.cancel()');
    assert.equal(await waitFor(host, 'window.blipNetRole() === 0', 10000), true);
    assert.equal(await waitFor(guest, 'window.blipNetRole() === 0', 20000), true);
  });
});
