import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const proto = require('../web/blip_net_proto.js');

// ---- room codes --------------------------------------------------------

test('genRoomCode produces a 4-digit, zero-padded string', () => {
  for (let i = 0; i < 200; i++) {
    const code = proto.genRoomCode();
    assert.equal(typeof code, 'string');
    assert.equal(code.length, 4);
    assert.match(code, /^[0-9]{4}$/, `not 4 digits: ${JSON.stringify(code)}`);
  }
});

test('genRoomCode covers the low end (zero-padding actually exercised)', () => {
  // Statistically near-certain in 5000 draws out of 10000 possible codes;
  // guards against a padStart regression silently dropping leading zeros.
  let sawLeadingZero = false;
  for (let i = 0; i < 5000; i++) {
    if (proto.genRoomCode()[0] === '0') { sawLeadingZero = true; break; }
  }
  assert.ok(sawLeadingZero, 'never saw a leading-zero code in 5000 draws');
});

test('normalizeRoomCode strips non-digits and truncates', () => {
  assert.equal(proto.normalizeRoomCode('12 34'), '1234');
  assert.equal(proto.normalizeRoomCode('12-34'), '1234');
  assert.equal(proto.normalizeRoomCode('  0042  '), '0042');
  assert.equal(proto.normalizeRoomCode('123456'), '1234'); // truncated, not rejected
  assert.equal(proto.normalizeRoomCode('abcd'), '');
  assert.equal(proto.normalizeRoomCode(''), '');
  assert.equal(proto.normalizeRoomCode(null), '');
  assert.equal(proto.normalizeRoomCode(undefined), '');
});

test('normalizeRoomCode coerces non-string input rather than throwing', () => {
  assert.equal(proto.normalizeRoomCode(1234), '1234');
  assert.equal(proto.normalizeRoomCode(42), '42');
});

test('isValidRoomCode accepts exactly 4 digits, post-normalization', () => {
  assert.equal(proto.isValidRoomCode('1234'), true);
  assert.equal(proto.isValidRoomCode('0000'), true);
  assert.equal(proto.isValidRoomCode('12 34'), true); // normalizes first
});

test('isValidRoomCode rejects the wrong shape', () => {
  assert.equal(proto.isValidRoomCode('123'), false);  // too short
  assert.equal(proto.isValidRoomCode('12a'), false);  // normalizes to '12' — still too short
  assert.equal(proto.isValidRoomCode('abcd'), false); // no digits at all
  assert.equal(proto.isValidRoomCode(''), false);
  assert.equal(proto.isValidRoomCode(null), false);
});

test('isValidRoomCode: normalizing to 4 digits is enough, even if the raw input was messier', () => {
  // normalizeRoomCode truncates rather than rejecting overlong input — a
  // deliberate leniency for a player fat-fingering a 5th digit — so a
  // 5-digit typo that normalizes down to a valid 4-digit code is accepted.
  // This documents that behavior; it is not a claim it's the only sane
  // choice, just the one this module makes.
  assert.equal(proto.isValidRoomCode('12345'), true);
});

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

// ---- bye messages --------------------------------------------------------

test('encodeBye/decodeBye round-trips', () => {
  assert.equal(proto.decodeBye(proto.encodeBye()), true);
});

test('decodeBye rejects everything that is not a bye packet', () => {
  assert.equal(proto.decodeBye(proto.encodeInput(true, false)), false);
  assert.equal(proto.decodeBye('not json'), false);
  assert.equal(proto.decodeBye('null'), false);
  assert.equal(proto.decodeBye(JSON.stringify({ v: 2, t: 'bye' })), false); // wrong version
});
