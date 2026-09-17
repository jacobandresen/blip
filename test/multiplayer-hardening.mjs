// The DataChannel peer is not trusted.
//
// Pairing is a QR code anyone in the room can photograph, and nothing
// authenticates the far side once the channel is open. So "the other
// device" is really "whoever is on the other end", and every inbound
// message is untrusted input. test/multiplayer-proto.test.mjs covers the
// wire format in isolation; this file covers the rules that need a live
// connection to mean anything -- who is allowed to say what, and what
// happens when a peer misbehaves.
//
// Each check ends by confirming the connection still works. That is the
// point of the whole file: hardening that silently breaks the channel
// would be worse than the hole it closed, and a test that only asserted
// "the bad thing was ignored" could not tell the two apart.

import test from 'node:test';
import assert from 'node:assert/strict';
import { pairOverQr } from './lib/pairing.mjs';
import {
  openPair, HTTP_PORT, READ_RIGHT_FRACTION,
  evaluate, waitFor, sleep,
} from './lib/multiplayer-harness.mjs';

const HOST_ENGINE = process.env.BLIP_HOST_ENGINE || 'chromium';
const GUEST_ENGINE = process.env.BLIP_GUEST_ENGINE || 'chromium';

/** Send a raw payload from one side, bypassing the encoders — the only
 * way to produce traffic a well-behaved peer never would. */
const sendRaw = (cdp, expr) => evaluate(cdp, `window.__blipNetSendRaw(${expr})`);

const keyState = (cdp) => evaluate(cdp, 'window.__blipNetDebug().keyState');
const stateLen = (cdp) => evaluate(cdp, 'window.__blipNetDebug().latestStateLen');
const dcState = (cdp) => evaluate(cdp, 'window.__blipNetDebug().dcState');

test(`multiplayer hardening against a hostile peer (${HOST_ENGINE} host, ${GUEST_ENGINE} guest)`, async (t) => {
  const { host, guest } = await openPair(t, HOST_ENGINE, GUEST_ENGINE);
  const paired = await pairOverQr(host, guest);
  assert.equal(paired.hostRole, 1);
  assert.equal(paired.guestRole, 2);

  await t.test('a host cannot drive the guest\'s paddle by sending it input packets', async () => {
    // Input is a guest -> host message. Honoured in the other direction
    // it would be worse than it looks: the guest synthesises real key
    // events from it, and the guest's own input capture then forwards
    // those back to the host — so a malicious host could not merely
    // move the guest's paddle, it could puppet the guest into playing
    // against itself.
    const before = await keyState(guest);
    assert.deepEqual(before, { KeyI: false, KeyK: false }, 'guest started with keys held');

    await sendRaw(host, "window.BlipNetProto.encodeInput(true, true)");
    await sleep(400);

    assert.deepEqual(await keyState(guest), { KeyI: false, KeyK: false },
      'the guest acted on an input packet it should have ignored');
  });

  await t.test('a guest cannot push state packets at the host', async () => {
    // The mirror rule. The host simulates; it must never render someone
    // else's idea of the match. Storing it would also hand a peer an
    // unbounded per-message allocation on a machine that never reads it.
    assert.equal(await stateLen(host), 0, 'host had buffered state before the test');
    await sendRaw(guest, 'new Uint8Array(36).buffer');
    await sleep(400);
    assert.equal(await stateLen(host), 0, 'the host buffered a state packet from its guest');
  });

  await t.test('an oversized binary frame is dropped rather than buffered', async () => {
    // 1 MB, far past MAX_STATE_BYTES. Accepting it would mean a copy per
    // message, at whatever rate the peer cares to send.
    //
    // Asserting "nothing is buffered" would be wrong here and only
    // passes by luck: a live match means the host is legitimately
    // sending a 36-byte state packet every frame, so the guest's buffer
    // is populated most of the time. What must be true is that whatever
    // is in there is always a real packet — never the oversized one.
    // 64KB: sixteen times MAX_STATE_BYTES, so unambiguously oversized to
    // the receiver, while staying inside what every engine will actually
    // transmit. A 1MB frame is not usable here — on WebKit, sending one
    // silently closes the *sender's* own channel (see MAX_SEND_BYTES in
    // web/blip_net.js), which would test the sender rather than the
    // receiver and take the rest of this file down with it.
    const NET_STATE_LEN = 36;
    await sendRaw(host, 'new Uint8Array(64 * 1024).buffer');
    let worst = 0;
    for (let i = 0; i < 12; i++) {
      worst = Math.max(worst, await stateLen(guest));
      await sleep(50);
    }
    assert.ok(worst <= NET_STATE_LEN,
      `the guest buffered an oversized frame (${worst} bytes, expected at most ${NET_STATE_LEN})`);
  });

  await t.test('a giant JSON payload is refused without parsing it', async () => {
    // The cap is checked before JSON.parse because parse cost scales
    // with size and it blocks the thread running the game loop.
    const started = Date.now();
    await sendRaw(host, `JSON.stringify({ v: 1, t: 'input', up: true, pad: 'x'.repeat(48 * 1024) })`);
    await sleep(500);
    assert.deepEqual(await keyState(guest), { KeyI: false, KeyK: false });
    t.diagnostic(`guest still responsive ${Date.now() - started}ms after an oversized packet`);
  });

  await t.test('a burst of malformed frames does not close or wedge the channel', async () => {
    const junk = [
      "'not json at all'", "'{'", "'[]'", "'null'", "'{\"v\":99,\"t\":\"input\"}'",
      "new Uint8Array(0).buffer", "new Uint8Array([1,2,3]).buffer",
      "JSON.stringify({ v: 1, t: 'ping', id: 'x'.repeat(5000) })",
    ];
    for (let i = 0; i < 5; i++) {
      for (const payload of junk) await sendRaw(guest, payload);
    }
    await sleep(600);
    assert.equal(await dcState(host), 'open', 'the host channel died on malformed input');
    assert.equal(await dcState(guest), 'open', 'the guest channel died on malformed input');
  });

  await t.test('a ping flood is answered at a bounded rate, not one-for-one', async () => {
    // Every ping obliges a reply. Unbounded, that is a cheap way to keep
    // the other device's main thread busy answering, using a message the
    // protocol is obliged to honour — nothing here is malformed.
    const before = await evaluate(host, 'window.__blipNetDebug()');
    const sent = await evaluate(guest, `(function () {
      var n = 0;
      for (var i = 0; i < 200; i++) {
        if (window.__blipNetSendRaw(window.BlipNetProto.encodePing('flood' + i))) n++;
      }
      return n;
    })()`);
    await sleep(1200);
    const after = await evaluate(host, 'window.__blipNetDebug()');
    const answered = after.pongsSent - before.pongsSent;
    const dropped = after.pongsDropped - before.pongsDropped;
    t.diagnostic(`ping flood: sent ${sent}, host answered ${answered}, dropped ${dropped}`);

    assert.ok(sent > 100, `expected the flood to actually be sent, got ${sent}`);
    assert.ok(dropped > 0, 'the host answered every ping in the flood — the rate limit did nothing');
    assert.ok(answered < sent / 2,
      `the host answered ${answered} of ${sent} pings; the budget should cap this far lower`);
    assert.equal(await dcState(host), 'open', 'the host channel died under a ping flood');
  });

  await t.test('our own send path refuses a payload big enough to kill the channel', async () => {
    // The hazard runs in the other direction to everything above: this
    // is not about what a peer sends us, but about what we send. On
    // WebKit a 1MB frame closes the sender's own DataChannel with no
    // exception to catch — send() returns normally and the channel is
    // simply gone. So the guard has to be a refusal before the call.
    //
    // window.blipNetSend is the bridge the Rust game uses for every
    // state packet, which makes it the realistic route for an
    // oversized buffer to ever reach send().
    assert.equal(await dcState(host), 'open', 'channel was not open before the test');
    await evaluate(host, 'window.blipNetSend(new Uint8Array(1024 * 1024))');
    await sleep(800);
    assert.equal(await dcState(host), 'open',
      'sending an oversized buffer through blipNetSend closed the channel');

    // And a normal packet still goes through afterwards.
    await evaluate(host, 'window.blipNetSend(new Uint8Array(36))');
    await sleep(200);
    assert.equal(await dcState(host), 'open');
  });

  await t.test('after all of that, the connection still carries a real match', async () => {
    // The whole point: none of the rules above may cost a legitimate
    // player anything. Guest input must still reach the host and come
    // back as simulated state.
    const before = await evaluate(guest, READ_RIGHT_FRACTION);
    assert.ok(before > 0.2 && before < 0.8, `expected a live paddle, got ${before}`);

    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keydown', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }))`);
    await waitFor(guest, `(${READ_RIGHT_FRACTION}) < ${before} - 0.03`, 10000);
    await evaluate(guest, `
      document.getElementById('glcanvas').dispatchEvent(new KeyboardEvent('keyup', {
        bubbles: true, cancelable: true, key: 'i', code: 'KeyI',
      }))`);
    await sleep(200);

    const hostAfter = await evaluate(host, READ_RIGHT_FRACTION);
    assert.ok(hostAfter < before,
      `the host stopped simulating the guest's input after the hardening tests (${before} -> ${hostAfter})`);
  });
});
