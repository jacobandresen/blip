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
