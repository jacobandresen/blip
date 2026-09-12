import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

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

// ---- app-level ping/pong (the JACK IN "CHECK" button/console command) ----

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
