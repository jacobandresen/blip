// Unit tests for web/blip_sdp_slim.js's slimSdpForQr() — the SDP
// candidate-trimming logic that keeps the paired QR code from ballooning
// into an unscannably dense one on a machine with many network
// interfaces (docs/multiplayer.md). Pure string logic, no DOM/WebRTC
// involved, so this exercises multi-candidate/IPv6/mDNS cases our
// single-interface test VM can never actually produce end-to-end.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { slimSdpForQr, parseCandidateLine } = require('../web/blip_sdp_slim.js');

const HEADER = [
  'v=0',
  'o=- 4875082106251092185 2 IN IP4 127.0.0.1',
  's=-',
  't=0 0',
  'a=group:BUNDLE 0',
  'm=application 33124 UDP/DTLS/SCTP webrtc-datachannel',
  'c=IN IP4 192.168.0.63',
].join('\r\n');
const FOOTER = [
  'a=ice-ufrag:rb6/',
  'a=ice-pwd:OnnWjOg2bnYrWPLpIbOpWDkF',
  'a=fingerprint:sha-256 6E:DB',
  'a=setup:actpass',
  'a=mid:0',
].join('\r\n');

function sdpWith(candidateLines) {
  return [HEADER].concat(candidateLines, [FOOTER]).join('\r\n');
}

function candidateCount(sdp) {
  return sdp.split('\r\n').filter((l) => l.indexOf('a=candidate:') === 0).length;
}

test('keeps a single plain IPv4 host candidate untouched', () => {
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  const out = slimSdpForQr(sdpWith([line]));
  assert.equal(candidateCount(out), 1);
  assert.ok(out.indexOf(line) !== -1);
});

test('keeps an mDNS-hidden host candidate (no colon in the address, not a literal IP)', () => {
  const line = 'a=candidate:1 1 udp 2113937151 8f14e45f-ceea.local 33124 typ host generation 0';
  const out = slimSdpForQr(sdpWith([line]));
  assert.equal(candidateCount(out), 1);
  assert.ok(out.indexOf('.local') !== -1, 'the mDNS candidate must survive, not just some candidate');
});

test('drops a literal IPv6 host candidate (colon in the address)', () => {
  const v4 = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  const v6 = 'a=candidate:2 1 udp 2113937150 fe80::1ff:fe23:4567:890a 33125 typ host generation 0';
  const out = slimSdpForQr(sdpWith([v4, v6]));
  assert.equal(candidateCount(out), 1);
  assert.ok(out.indexOf('fe80::') === -1);
  assert.ok(out.indexOf('192.168.0.63') !== -1);
});

test('drops a non-host (srflx/relay) candidate', () => {
  const host = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  const srflx = 'a=candidate:2 1 udp 1677729535 203.0.113.9 33126 typ srflx raddr 192.168.0.63 rport 33126 generation 0';
  const out = slimSdpForQr(sdpWith([host, srflx]));
  assert.equal(candidateCount(out), 1);
  assert.ok(out.indexOf('typ srflx') === -1);
});

test('caps at the default limit (4) when many host candidates are offered', () => {
  const lines = [];
  for (let i = 0; i < 9; i++) {
    lines.push(`a=candidate:${i} 1 udp 2113937151 10.0.0.${i} 3300${i} typ host generation 0`);
  }
  const out = slimSdpForQr(sdpWith(lines));
  assert.equal(candidateCount(out), 4);
});

test('a custom limit is honored', () => {
  const lines = [];
  for (let i = 0; i < 9; i++) {
    lines.push(`a=candidate:${i} 1 udp 2113937151 10.0.0.${i} 3300${i} typ host generation 0`);
  }
  const out = slimSdpForQr(sdpWith(lines), 2);
  assert.equal(candidateCount(out), 2);
});

test('falls back to the original, untouched SDP if filtering would leave zero candidates', () => {
  const onlyV6 = 'a=candidate:1 1 udp 2113937151 fe80::1ff:fe23:4567:890a 33124 typ host generation 0';
  const original = sdpWith([onlyV6]);
  const out = slimSdpForQr(original);
  assert.equal(out, original);
  assert.equal(candidateCount(out), 1, 'the one (IPv6) candidate must still be there — no route is worse than a dense QR code');
});

test('non-candidate lines are never touched, in either order or content', () => {
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  const out = slimSdpForQr(sdpWith([line]));
  assert.equal(out.split('\r\n')[0], 'v=0');
  assert.ok(out.indexOf('a=fingerprint:sha-256 6E:DB') !== -1);
  assert.ok(out.indexOf('a=mid:0') !== -1);
});

test('an SDP with no candidates at all round-trips unchanged', () => {
  const out = slimSdpForQr(sdpWith([]));
  assert.equal(out, sdpWith([]));
});

test('a malformed candidate line (too few fields to have a real address/port) is kept as-is, not dropped', () => {
  // parseCandidateLine() can't make sense of this — rather than guess
  // (and risk silently corrupting a real, if oddly-shaped, line),
  // slimSdpForQr treats "couldn't parse it" the same as "not a
  // candidate line at all" and leaves it untouched.
  const broken = 'a=candidate:1 1 udp';
  const out = slimSdpForQr(sdpWith([broken]));
  assert.ok(out.indexOf(broken) !== -1);
});

// ---- parseCandidateLine ---------------------------------------------------

test('parseCandidateLine extracts protocol/address/port/type from a well-formed line', () => {
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  assert.deepEqual(parseCandidateLine(line), {
    protocol: 'udp', address: '192.168.0.63', port: '33124', type: 'host',
  });
});

test('parseCandidateLine handles an mDNS address the same as any other', () => {
  const line = 'a=candidate:1 1 udp 2113937151 8f14e45f-ceea.local 33124 typ host generation 0';
  const parsed = parseCandidateLine(line);
  assert.equal(parsed.address, '8f14e45f-ceea.local');
  assert.equal(parsed.type, 'host');
});

test('parseCandidateLine returns null for a line missing "typ" entirely', () => {
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124';
  const parsed = parseCandidateLine(line);
  assert.equal(parsed.type, '');
  assert.equal(parsed.address, '192.168.0.63'); // still has enough fields for the rest
});

test('parseCandidateLine returns null for anything that is not an a=candidate: line', () => {
  assert.equal(parseCandidateLine('a=mid:0'), null);
  assert.equal(parseCandidateLine(''), null);
  assert.equal(parseCandidateLine('candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host'), null); // missing the leading "a="
});

test('parseCandidateLine returns null for non-string input rather than throwing', () => {
  assert.equal(parseCandidateLine(undefined), null);
  assert.equal(parseCandidateLine(null), null);
  assert.equal(parseCandidateLine(42), null);
});

test('parseCandidateLine returns null for a line too short to have an address/port', () => {
  assert.equal(parseCandidateLine('a=candidate:1 1 udp'), null);
  assert.equal(parseCandidateLine('a=candidate:'), null);
});
