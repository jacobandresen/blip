// Two-device Rally multiplayer across real browser *engines*, driven by
// Playwright.
//
// Why this exists alongside test/multiplayer.mjs: that suite runs
// Chromium on both sides. But the second device in the shipped feature
// is usually a phone, and on iOS every browser is WebKit — so the half
// of this feature most likely to be used was, until this file, covered
// by no test at all. WebRTC is exactly where that gap bites: SDP shape,
// ICE candidate generation, and DataChannel timing are engine
// behaviours, not spec constants.
//
// The engines are selectable so the same assertions cover three
// pairings:
//
//   BLIP_HOST_ENGINE=webkit   BLIP_GUEST_ENGINE=webkit    (default; both WebKit)
//   BLIP_HOST_ENGINE=chromium BLIP_GUEST_ENGINE=webkit    (desktop Chrome <-> iPhone)
//   BLIP_HOST_ENGINE=webkit   BLIP_GUEST_ENGINE=chromium  (Mac Safari <-> Android Chrome)
//
// Signaling uses the scan-injection hook, for the reason spelled out in
// test/lib/pairing.mjs: the QR *payload* still makes the full round trip
// through render, validation, and applyDescription — only the optics are
// skipped. Camera decode itself is covered on Chromium by
// test/multiplayer.mjs's BLIP_MULTIPLAYER_REAL_CAMERA mode; WebKit has no
// fake-camera equivalent to drive headlessly.

import test from 'node:test';
import assert from 'node:assert/strict';
import { pairOverQr } from './lib/pairing.mjs';
import {
  openPage, openPair, HTTP_PORT, loadRally, READ_RIGHT_FRACTION,
  openModal, clickHsBtn, getStatusText, pollUntil, QR_READY,
  evaluate, waitFor, sleep,
} from './lib/multiplayer-harness.mjs';

const HOST_ENGINE = process.env.BLIP_HOST_ENGINE || 'webkit';
const GUEST_ENGINE = process.env.BLIP_GUEST_ENGINE || 'webkit';

test(`two-device Rally over WebRTC: host=${HOST_ENGINE} guest=${GUEST_ENGINE}`, async (t) => {
  const { host, guest } = await openPair(t, HOST_ENGINE, GUEST_ENGINE);

  let paired;

  await t.test('the QR exchange completes and the DataChannel opens on both sides', async () => {
    paired = await pairOverQr(host, guest);
    assert.equal(paired.hostRole, 1, 'host should report role 1 once its DataChannel opens');
    assert.equal(paired.guestRole, 2, 'guest should report role 2 once its DataChannel opens');
  });

  await t.test('the offer and answer really were this engine pair\'s own SDP', async () => {
    // Guards against a pairing that "succeeds" because both sides
    // silently reused a stale or synthetic description: the payloads
    // must be distinct, non-empty, and each carry ICE candidates, which
    // is what blip_sdp_slim.js's validation is there to enforce.
    assert.notEqual(paired.offer, paired.answer, 'offer and answer must differ');
    for (const [label, text] of [['offer', paired.offer], ['answer', paired.answer]]) {
      assert.ok(text.length > 40, `${label} payload looks empty (${text.length} bytes)`);
      // The QR carries the compact form, not raw SDP (see packForQr in
      // web/blip_sdp_slim.js) — the whole point of which is that it does
      // *not* contain SDP boilerplate. What must be true is that it is
      // the compact format and names at least one reachable route.
      assert.ok(text.startsWith('B1|'), `${label} is not a compact payload: ${text.slice(0, 40)}`);
      const routes = text.split('|')[5] || '';
      assert.ok(/[^:]+:\d+/.test(routes), `${label} carries no route: ${JSON.stringify(routes)}`);
      // The compact form exists to keep the code scannable; if it ever
      // grows back toward SDP size, the QR quietly gets harder to scan.
      assert.ok(text.length < 400,
        `${label} is ${text.length} bytes — the compact payload should be ~100`);
    }
  });

  await t.test('the connection is a real peer-to-peer DataChannel, not a half-open one', async () => {
    for (const [label, cdp] of [['host', host], ['guest', guest]]) {
      const dbg = await evaluate(cdp, 'window.__blipNetDebug()');
      assert.equal(dbg.dcState, 'open', `${label} DataChannel is ${dbg.dcState}`);
      assert.ok(['connected', 'completed'].includes(dbg.pcIceConnectionState),
        `${label} ICE state is ${dbg.pcIceConnectionState}`);
    }
  });

  await t.test('the match actually starts on both devices (Rust picks up the net role)', async () => {
    assert.ok(await waitFor(host, `(${READ_RIGHT_FRACTION}) > 0.3`, 15000),
      'host paddle never left the Title-screen position');
    assert.ok(await waitFor(guest, `(${READ_RIGHT_FRACTION}) > 0.3`, 15000),
      'guest paddle never left the Title-screen position');
  });

  await t.test('guest input reaches the host and the resulting state syncs back', async () => {
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.3 && before < 0.7, `expected a centered paddle to start, got ${before}`);

    const press = (type) => evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('${type}', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }))`);

    // Hold until the guest's *mirrored* view moves — that alone proves a
    // full guest -> host -> guest round trip, since the guest renders
    // only what the host simulates and sends back. Then release fast:
    // both paddles are in continuous motion while the key is held, so a
    // comparison taken mid-motion catches the two sides one round trip
    // apart by construction. (Same reasoning as test/multiplayer.mjs.)
    await press('keydown');
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 10000);
    await press('keyup');
    await sleep(200); // let the last state packets in flight land on both sides

    const guestAfter = await evaluate(guest, READ_RIGHT_FRACTION);
    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(guestAfter < before,
      `guest's mirrored paddle should show it moved up (was ${before}, now ${guestAfter})`);
    assert.ok(hostAfter < before,
      `host should have actually simulated the remote input (was ${before}, now ${hostAfter})`);
    assert.ok(Math.abs(guestAfter - hostAfter) < 0.05,
      `host and guest should agree on the settled paddle position (host ${hostAfter}, guest ${guestAfter})`);
  });

  await t.test('state keeps flowing host -> guest continuously, not just once', async () => {
    // A DataChannel that opens, delivers one burst and stalls would pass
    // every check above. What it could not do is keep the guest's view
    // *moving*: the guest runs no simulation of its own, so every frame
    // of motion it renders came from a state packet that just arrived.
    //
    // Sampled as an effect rather than by reading __blipNetDebug()'s
    // latestStateLen, which is 0 almost every time you look at it —
    // blipNetPoll() hands each packet to the game and immediately nulls
    // it ("only ever hand back a packet once"), so that counter races
    // the game loop and says nothing about whether the stream is alive.
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: 'k', code: 'KeyK',
      }))`);
    const samples = [];
    for (let i = 0; i < 6; i++) {
      samples.push(await evaluate(guest, READ_RIGHT_FRACTION));
      await sleep(120);
    }
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: 'k', code: 'KeyK',
      }))`);

    const distinct = new Set(samples.map((v) => Math.round(v * 1000))).size;
    assert.ok(distinct >= 3,
      `guest's view should keep moving while input is held — got ${distinct} distinct positions in ${JSON.stringify(samples)}`);
  });

  await t.test('a clean disconnect is observed on both sides', async () => {
    await evaluate(host, 'window.BlipNet.cancel()');
    assert.equal(await waitFor(host, 'window.blipNetRole() === 0', 8000), true);
    assert.equal(await waitFor(guest, 'window.blipNetRole() === 0', 12000), true);
  });
});

// The original bug was not "hosting fails" but "hosting hangs": with no
// candidates, the modal sat on "Creating code…" until the 60s connect
// timeout, saying nothing. Refusing the camera is the reachable way into
// that region of the code, so this pins the property that actually
// matters — hosting reaches a *terminal, legible* state well inside the
// ICE gather timeout — without asserting which one.
//
// Which one it is genuinely varies on this engine. Once warmUpIceMedia()
// has run, headless WebKit will often gather and render an offer even
// though the getUserMedia it made was rejected; sometimes it gathers
// nothing and the run ends on the camera message instead. Asserting
// either outcome specifically produces a test that fails on its own
// engine's coin flip, so this accepts both and rejects only the hang.
test('WebKit: refusing the camera never leaves hosting stuck without a message', async (t) => {
  // No 'camera' in permissions — getUserMedia is refused, exactly as it
  // is for a player who taps "Don't Allow".
  const { cdp: host } = await openPage(t, 'webkit', { permissions: [] });

  await loadRally(host);
  assert.equal(await evaluate(host, `
    new Promise(function (resolve) {
      navigator.mediaDevices.getUserMedia({ video: true }).then(
        function (s) { s.getTracks().forEach(function (t) { t.stop(); }); resolve('granted'); },
        function (e) { resolve(e.name); });
    })`), 'NotAllowedError', 'this test is meaningless unless the camera is actually refused');

  await openModal(host);
  await clickHsBtn(host, 'HOST');

  const outcome = await pollUntil(async () => {
    if (await evaluate(host, QR_READY)) return 'offer rendered';
    const s = await getStatusText(host);
    if (s === 'Allow camera access to connect.') return s;
    if (s && /Could not connect|Timed out/.test(s)) return `unhelpful: ${s}`;
    return null;
  }, 25000, 400);

  assert.ok(['offer rendered', 'Allow camera access to connect.'].includes(outcome),
    `hosting with no camera should end in an offer or a camera message, got: ${outcome}`);
});
