import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const require = createRequire(import.meta.url);
const proto = require('../web/blip_net_proto.js');

// ---- input messages ------------------------------------------------------

test('encodeInput/decodeInput round-trips all four up/down combinations', () => {
  for (const up of [true, false]) {
    for (const down of [true, false]) {
      const wire = proto.encodeInput(up, down);
      assert.equal(typeof wire, 'string');
      const back = proto.decodeInput(wire);
      assert.deepEqual(back, { up, down });
    }
  }
});

test('encodeInput coerces truthy/falsy non-booleans', () => {
  const wire = proto.encodeInput(1, 0);
  assert.deepEqual(proto.decodeInput(wire), { up: true, down: false });
  const wire2 = proto.encodeInput('yes', null);
  assert.deepEqual(proto.decodeInput(wire2), { up: true, down: false });
});

test('encodeInput output carries the current protocol version', () => {
  const parsed = JSON.parse(proto.encodeInput(true, false));
  assert.equal(parsed.v, proto.PROTO_VERSION);
  assert.equal(parsed.t, 'input');
});

test('decodeInput rejects malformed JSON without throwing', () => {
  assert.equal(proto.decodeInput('not json'), null);
  assert.equal(proto.decodeInput('{"up": true,'), null); // truncated
  assert.equal(proto.decodeInput(''), null);
  assert.equal(proto.decodeInput(undefined), null);
});

test('decodeInput rejects well-formed JSON that is not an input packet', () => {
  assert.equal(proto.decodeInput('null'), null);
  assert.equal(proto.decodeInput('42'), null);
  assert.equal(proto.decodeInput('"just a string"'), null);
  assert.equal(proto.decodeInput('[1,2,3]'), null); // array: typeof is 'object' but has no .t
  assert.equal(proto.decodeInput('{}'), null); // missing t
  assert.equal(proto.decodeInput('{"t":"state"}'), null); // wrong t
});

test('decodeInput rejects a version it does not understand', () => {
  assert.equal(proto.decodeInput(JSON.stringify({ v: 2, t: 'input', up: true, down: false })), null);
  assert.equal(proto.decodeInput(JSON.stringify({ t: 'input', up: true, down: false })), null); // v missing entirely
});

test('decodeInput returns exactly { up, down } and ignores extra fields', () => {
  const back = proto.decodeInput(JSON.stringify({
    v: 1, t: 'input', up: true, down: false, extra: 'nope', roomCode: '1234',
  }));
  assert.deepEqual(Object.keys(back).sort(), ['down', 'up']);
  assert.equal(back.extra, undefined);
});

test('the module never exports the removed bye-message functions', () => {
  // encodeBye/decodeBye were designed but never actually wired into
  // blip_net.js's message handling or ever sent anywhere — dead code,
  // trimmed. This just guards against it quietly reappearing unused.
  assert.equal(proto.encodeBye, undefined);
  assert.equal(proto.decodeBye, undefined);
});

// ---- app-level ping/pong diagnostic ----------------------------------------

test('encodePing/decodePing round-trips an id', () => {
  const wire = proto.encodePing('p1');
  assert.equal(typeof wire, 'string');
  assert.deepEqual(proto.decodePing(wire), { id: 'p1' });
});

test('encodePong/decodePong round-trips an id', () => {
  const wire = proto.encodePong('p1');
  assert.deepEqual(proto.decodePong(wire), { id: 'p1' });
});

test('encodePing/encodePong coerce a non-string id to a string', () => {
  assert.deepEqual(proto.decodePing(proto.encodePing(42)), { id: '42' });
  assert.deepEqual(proto.decodePong(proto.encodePong(42)), { id: '42' });
});

test('encodePing/encodePong output carries the current protocol version', () => {
  assert.equal(JSON.parse(proto.encodePing('p1')).v, proto.PROTO_VERSION);
  assert.equal(JSON.parse(proto.encodePong('p1')).v, proto.PROTO_VERSION);
});

test('decodePing/decodePong reject malformed or foreign JSON without throwing', () => {
  assert.equal(proto.decodePing('not json'), null);
  assert.equal(proto.decodePing(''), null);
  assert.equal(proto.decodePing(undefined), null);
  assert.equal(proto.decodePong('not json'), null);
});

test('decodePing/decodePong reject the wrong message type, id shape, or version', () => {
  assert.equal(proto.decodePing(proto.encodeInput(true, false)), null); // an input packet, not a ping
  assert.equal(proto.decodePing(proto.encodePong('p1')), null); // a pong, not a ping
  assert.equal(proto.decodePong(proto.encodePing('p1')), null); // a ping, not a pong
  assert.equal(proto.decodePing(JSON.stringify({ v: 1, t: 'ping', id: 7 })), null); // id not a string
  assert.equal(proto.decodePing(JSON.stringify({ v: 2, t: 'ping', id: 'p1' })), null); // wrong version
});

test('decodeInput still rejects a ping/pong packet (distinct t)', () => {
  assert.equal(proto.decodeInput(proto.encodePing('p1')), null);
  assert.equal(proto.decodeInput(proto.encodePong('p1')), null);
});

// ---- hardening: the decoders' side of an untrusted peer ------------------
// Pairing is a QR code anyone in the room can photograph and nothing
// authenticates the peer afterwards, so every function above is parsing
// hostile input by default. These pin the limits that make that safe.

test('parsing is refused above MAX_PACKET_CHARS, before JSON.parse runs', () => {
  const pad = 'x'.repeat(proto.MAX_PACKET_CHARS);
  const huge = JSON.stringify({ v: proto.PROTO_VERSION, t: 'input', up: true, pad });
  assert.ok(huge.length > proto.MAX_PACKET_CHARS);
  assert.equal(proto.decodeInput(huge), null);
  assert.equal(proto.decodePing(huge), null);
  assert.equal(proto.decodePong(huge), null);
});

test('a packet right at the size limit is still accepted', () => {
  // The cap must bound abuse without clipping a legitimate packet, so
  // pin both sides of the boundary, not just the rejecting one.
  const base = { v: proto.PROTO_VERSION, t: 'input', up: true, down: false, pad: '' };
  const pad = 'x'.repeat(proto.MAX_PACKET_CHARS - JSON.stringify(base).length);
  const wire = JSON.stringify({ ...base, pad });
  assert.equal(wire.length, proto.MAX_PACKET_CHARS);
  assert.deepEqual(proto.decodeInput(wire), { up: true, down: false });
});

test('an oversized ping id is refused, so a pong cannot be used to amplify', () => {
  const id = 'x'.repeat(proto.MAX_ID_CHARS + 1);
  assert.equal(proto.decodePing(JSON.stringify({ v: proto.PROTO_VERSION, t: 'ping', id })), null);
  assert.equal(proto.decodePong(JSON.stringify({ v: proto.PROTO_VERSION, t: 'pong', id })), null);
  const ok = 'x'.repeat(proto.MAX_ID_CHARS);
  assert.deepEqual(proto.decodePing(JSON.stringify({ v: proto.PROTO_VERSION, t: 'ping', id: ok })), { id: ok });
});

test('an empty ping id is refused', () => {
  assert.equal(proto.decodePing(JSON.stringify({ v: proto.PROTO_VERSION, t: 'ping', id: '' })), null);
});

test('ids this module generates fit well inside the accepted length', () => {
  // Guards the cap against the generator: shortening MAX_ID_CHARS below
  // what encodePing produces would make every real ping undecodable.
  const id = 'p999-' + Date.now();
  assert.deepEqual(proto.decodePing(proto.encodePing(id)), { id });
  assert.ok(id.length < proto.MAX_ID_CHARS);
});

test('arrays and primitives are not packets', () => {
  for (const raw of ['[]', '[{"v":1,"t":"input"}]', '1', '"input"', 'true', 'null']) {
    assert.equal(proto.decodeInput(raw), null, raw);
    assert.equal(proto.decodePing(raw), null, raw);
  }
});

test('non-string input is refused without throwing', () => {
  for (const raw of [undefined, null, 42, {}, [], new ArrayBuffer(8)]) {
    assert.equal(proto.decodeInput(raw), null);
    assert.equal(proto.decodePing(raw), null);
    assert.equal(proto.decodePong(raw), null);
  }
});

test('a __proto__ key in a packet cannot reach Object.prototype', () => {
  // JSON.parse makes __proto__ an ordinary own property rather than
  // invoking the setter, and nothing here spreads or merges the parsed
  // object. This pins that, because the day someone "simplifies" a
  // decoder into an object spread is the day it stops being true.
  const raw = '{"v":1,"t":"input","up":true,"__proto__":{"polluted":"yes"}}';
  assert.deepEqual(proto.decodeInput(raw), { up: true, down: false });
  assert.equal({}.polluted, undefined);
  assert.equal(Object.prototype.polluted, undefined);
});

test('decoders never throw, whatever the payload', () => {
  const nasty = [
    '{', '}', '[', 'undefined', 'NaN', '{"v":1,"t":"input","up":{"toString":1}}',
    '{"v":"1","t":"input"}', '{"v":1}', '{"t":"input"}', ' ', '{"v":1,"t":"ping","id":null}',
    '{"v":1,"t":"ping","id":{}}', '{"v":1,"t":"ping","id":["a"]}', '{"v":1.0000001,"t":"input"}',
  ];
  for (const raw of nasty) {
    assert.doesNotThrow(() => proto.decodeInput(raw), raw);
    assert.doesNotThrow(() => proto.decodePing(raw), raw);
    assert.doesNotThrow(() => proto.decodePong(raw), raw);
  }
});

// ---- the browser half of the dual-mode export ----------------------------
// Every module under web/ ends with the same idiom: attach to
// `module.exports` under Node, or to `window` in a browser. Node tests only
// ever take the first branch, so the second — the one that actually ships,
// and the only one a player's browser runs — was the sole uncovered code in
// all three pure-logic modules.
//
// Evaluating the source with no `module` in scope and a stand-in `window`
// takes the browser path for real, which both covers it and checks the
// thing that matters: that loading the file as a plain <script> puts the
// expected API on the global.
test('each shipped module attaches its API to window when loaded as a script', () => {
  const cases = [
    ['blip_net_proto.js', 'BlipNetProto', ['encodeInput', 'decodeInput', 'encodePing', 'decodePong']],
    ['blip_sdp_slim.js', 'BlipSdpSlim', ['slimSdpForQr', 'parseCandidateLine', 'packForQr', 'unpackFromQr']],
    ['blip_qr_heuristic.js', 'BlipQrHeuristic', []],
  ];
  for (const [file, globalName, expected] of cases) {
    const src = readFileSync(new URL(`../web/${file}`, import.meta.url), 'utf8');
    const fakeWindow = {};
    // No `module` parameter, so `typeof module` is 'undefined' inside and
    // the browser branch is the one that runs.
    new Function('window', src)(fakeWindow);
    assert.ok(fakeWindow[globalName], `${file} did not define window.${globalName}`);
    for (const fn of expected) {
      assert.equal(typeof fakeWindow[globalName][fn], 'function',
        `window.${globalName}.${fn} missing after loading ${file} as a script`);
    }
  }
});
