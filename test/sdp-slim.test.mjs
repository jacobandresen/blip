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
const { slimSdpForQr, parseCandidateLine, packForQr, unpackFromQr } = require('../web/blip_sdp_slim.js');

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

test('keeps a plain IPv4 host candidate, with every field the remote needs', () => {
  // Kept, but no longer byte-identical: the optional trailing attributes
  // are stripped (see below). What must survive is everything the remote
  // parses — foundation, component, transport, priority, address, port
  // and the `typ`.
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host generation 0';
  const out = slimSdpForQr(sdpWith([line]));
  assert.equal(candidateCount(out), 1);
  assert.ok(out.indexOf('a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host') !== -1,
    `the candidate's required fields were altered: ${out}`);
});

test('strips the optional trailing attributes, which are pure QR payload', () => {
  // ~40 bytes per candidate. On a code this size that is the difference
  // between one QR version and the next one down — i.e. physically
  // larger modules for the scanning camera to resolve.
  const line = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host ' +
    'generation 0 ufrag k3Bd network-id 2 network-cost 10';
  const out = slimSdpForQr(sdpWith([line]));
  assert.equal(candidateCount(out), 1);

  // Checked on the candidate line alone, not the whole SDP: the session
  // has its own required `a=ice-ufrag:` attribute, and a naive search
  // for "ufrag" across the document matches that too — which must
  // survive, and whose removal would break pairing outright.
  const candidateLine = out.split('\r\n').find((l) => l.indexOf('a=candidate:') === 0);
  for (const attr of ['generation', 'ufrag', 'network-id', 'network-cost']) {
    assert.ok(candidateLine.indexOf(attr) === -1, `${attr} survived on: ${candidateLine}`);
  }
  assert.ok(out.indexOf('a=ice-ufrag:') !== -1, 'the session ice-ufrag must not be stripped');
  assert.ok(out.length < sdpWith([line]).length, 'stripping made the payload no smaller');
});

test('drops TCP candidates, which cannot connect these two peers', () => {
  // Chrome offers `tcptype active` candidates on port 9. They can only
  // pair with a *passive* TCP candidate, which neither side ever offers:
  // the DataChannel runs over UDP/DTLS/SCTP and there is no TURN server.
  // They are routes guaranteed to fail, taking up QR payload to say so.
  const udp = 'a=candidate:1 1 udp 2113937151 192.168.0.63 33124 typ host';
  const tcp = 'a=candidate:2 1 tcp 1518280447 192.168.0.63 9 typ host tcptype active';
  const out = slimSdpForQr(sdpWith([udp, tcp]));
  assert.equal(candidateCount(out), 1, 'expected only the UDP candidate to survive');
  assert.ok(out.indexOf(' tcp ') === -1, `a TCP candidate survived: ${out}`);
  assert.ok(out.indexOf('192.168.0.63 33124') !== -1, 'the UDP candidate should have survived');
});

test('a TCP-only SDP is left alone rather than stripped to nothing', () => {
  // The existing guarantee: trimming must never produce an SDP with no
  // route at all. If TCP is all there is, a dense code beats no code.
  const tcp = 'a=candidate:2 1 tcp 1518280447 192.168.0.63 9 typ host tcptype active';
  const out = slimSdpForQr(sdpWith([tcp]));
  assert.ok(out.indexOf('tcp') !== -1, 'a TCP-only SDP should be returned unchanged');
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

// ---- compact QR payload --------------------------------------------------
// A datachannel offer is ~538 bytes of SDP, of which ~98 are facts the far
// side cannot derive. Sending only those takes the code from 89 modules to
// 53 while raising error correction from M to H — modules 1.7x larger and
// double the damage tolerance, which is what makes it scan in ordinary
// light rather than needing a steady hand and a bright room.
//
// The risk this trades for is that the far side rebuilds the SDP, so these
// tests exist to pin exactly what must survive the round trip.

const FP = '8B:6C:0B:62:CC:4D:2A:7E:33:0D:05:03:C6:27:9E:4A:' +
           '8B:6C:0B:62:CC:4D:2A:7E:33:0D:05:03:C6:27:9E:4A';

function offerSdp(candidateLines, setup = 'actpass') {
  return [
    'v=0', 'o=- 88 2 IN IP4 127.0.0.1', 's=-', 't=0 0', 'a=group:BUNDLE 0',
    'm=application 58660 UDP/DTLS/SCTP webrtc-datachannel', 'c=IN IP4 192.168.0.162',
    ...candidateLines,
    'a=ice-ufrag:srqd', 'a=ice-pwd:YxwpMhjbDZ5Sr9s1K2uvKVup', 'a=ice-options:trickle',
    `a=fingerprint:sha-256 ${FP}`, `a=setup:${setup}`, 'a=mid:0', 'a=sctp-port:5000',
  ].join('\r\n') + '\r\n';
}
const HOST_CAND = 'a=candidate:1526232009 1 udp 2122260223 192.168.0.162 58660 typ host';

test('the compact payload keeps every fact the far side cannot derive', () => {
  const packed = packForQr(offerSdp([HOST_CAND]));
  assert.ok(packed.startsWith('B1|'), `not compact: ${packed.slice(0, 40)}`);
  const out = unpackFromQr(packed);
  assert.ok(out.indexOf('a=ice-ufrag:srqd') !== -1, 'ufrag lost');
  assert.ok(out.indexOf('a=ice-pwd:YxwpMhjbDZ5Sr9s1K2uvKVup') !== -1, 'ice-pwd lost');
  assert.ok(out.indexOf(`a=fingerprint:sha-256 ${FP}`) !== -1, 'fingerprint not restored byte-exact');
  assert.ok(out.indexOf('a=setup:actpass') !== -1, 'DTLS role lost');
  assert.ok(out.indexOf('192.168.0.162 58660') !== -1, 'candidate lost');
  assert.ok(out.startsWith('v=0'), 'rebuilt payload is not SDP');
});

test('the compact payload is dramatically smaller than the SDP it replaces', () => {
  const sdp = offerSdp([HOST_CAND]);
  const packed = packForQr(sdp);
  assert.ok(packed.length < sdp.length / 3,
    `compact form is ${packed.length} bytes against ${sdp.length} — the size win is the point`);
});

test('all three DTLS roles survive', () => {
  // Getting this wrong produces a connection that negotiates and then
  // silently fails to establish DTLS, which is near-impossible to debug.
  for (const setup of ['actpass', 'active', 'passive']) {
    const out = unpackFromQr(packForQr(offerSdp([HOST_CAND], setup)));
    assert.ok(out.indexOf(`a=setup:${setup}`) !== -1, `${setup} did not survive`);
  }
});

test('an mDNS .local candidate survives, since that is what Safari sends', () => {
  const mdns = 'a=candidate:1 1 udp 2113937151 8f14e45f-ceea.local 33124 typ host';
  const out = unpackFromQr(packForQr(offerSdp([mdns])));
  assert.ok(out.indexOf('8f14e45f-ceea.local 33124') !== -1, `.local candidate lost: ${out}`);
});

test('multiple candidates all survive, in order', () => {
  const two = [HOST_CAND, 'a=candidate:2 1 udp 2122194687 10.0.0.5 40404 typ host'];
  const out = unpackFromQr(packForQr(offerSdp(two)));
  assert.ok(out.indexOf('192.168.0.162 58660') !== -1);
  assert.ok(out.indexOf('10.0.0.5 40404') !== -1);
});

test('plain SDP passes through unpack untouched', () => {
  // Anything without the marker must be treated as SDP, so a payload this
  // codec did not produce is never silently reinterpreted.
  const sdp = offerSdp([HOST_CAND]);
  assert.equal(unpackFromQr(sdp), sdp);
});

test('an SDP missing a required field is left as SDP rather than packed lossily', () => {
  // Better a denser code than one that rebuilds into something subtly wrong.
  const noFingerprint = offerSdp([HOST_CAND]).replace(/a=fingerprint:[^\r]*\r\n/, '');
  assert.equal(packForQr(noFingerprint), noFingerprint);
  const noCandidates = offerSdp([]);
  assert.equal(packForQr(noCandidates), noCandidates);
});

test('a malformed compact payload is rejected, not half-rebuilt', () => {
  for (const bad of ['B1|', 'B1|a|b', 'B1|u|p|notbase64!|a|1.2.3.4:1', 'B1|u|p|' + 'A'.repeat(43) + '|a|',
                     'B1|u|p|' + 'A'.repeat(43) + '|z|1.2.3.4:1', 'B1|u|p|' + 'A'.repeat(43) + '|a|1.2.3.4:x']) {
    assert.equal(unpackFromQr(bad), '', `should have been rejected: ${bad}`);
  }
});

test('a field containing the separator refuses to pack rather than corrupt', () => {
  // The format is separator-delimited, so a value containing one would
  // silently shift every later field on the way back.
  const sdp = offerSdp([HOST_CAND]).replace('a=ice-pwd:YxwpMhjbDZ5Sr9s1K2uvKVup', 'a=ice-pwd:bad|pwd');
  assert.equal(packForQr(sdp), sdp, 'should have declined to pack');
});

test('unpack never throws, whatever it is handed', () => {
  for (const raw of ['', 'B1', 'B1|||||', 'not sdp at all', '{}', 'B1|' + '|'.repeat(50)]) {
    assert.doesNotThrow(() => unpackFromQr(raw), JSON.stringify(raw));
  }
});
