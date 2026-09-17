/* Trim an SDP's ICE candidates down to what actually matters for the
 * same-room/same-WiFi pairing case (docs/multiplayer.md) before it gets
 * encoded as a QR code — see web/blip_net.js's slimSdpForQr() call sites.
 *
 * A real machine can advertise far more candidates than a clean test VM's
 * one network interface: a VPN adapter, a bridged/virtual NIC from some
 * other app, an IPv6 address alongside the IPv4 one, etc. Every extra
 * candidate is another `a=candidate:` line in the SDP, and every extra
 * byte pushes the QR code to a denser version — smaller modules, harder
 * for a real camera (an older/lower-res one especially) to resolve. This
 * only affects what gets *transmitted* in the code; the RTCPeerConnection
 * that generated the SDP keeps its own full candidate set regardless.
 *
 * Also exports parseCandidateLine() so candidate filtering and tests share
 * one parser instead of duplicating the SDP syntax handling.
 *
 * Pure string logic, no DOM/WebRTC types touched — dual-exported (Node
 * for test/sdp-slim.test.mjs, `window.BlipSdpSlim` in the browser) the
 * same way web/blip_net_proto.js is.
 */
(function (root) {
  'use strict';

  var DEFAULT_MAX_CANDIDATES = 4;

  /** Parse one SDP line as an `a=candidate:` line — the fields
   * slimSdpForQr needs for filtering:
   *   a=candidate:<foundation> <component> <protocol> <priority> <address> <port> typ <type> ...
   * Returns `null` for anything that isn't a well-formed candidate line
   * (wrong prefix, or too few fields to have a usable address/port) —
   * callers should treat that the same as "not a candidate line at all"
   * rather than guess at a broken one. */
  function parseCandidateLine(line) {
    if (typeof line !== 'string' || line.indexOf('a=candidate:') !== 0) return null;
    var parts = line.split(' ');
    if (parts.length < 6) return null;
    var typIndex = parts.indexOf('typ');
    return {
      protocol: parts[2] || '',
      address: parts[4] || '',
      port: parts[5] || '',
      // `typ` (and the type after it) can in principle be absent from an
      // otherwise well-formed-looking line — every real candidate line
      // has one, but this is untrusted-ish text (scanned off a QR code
      // originally), so degrade to an empty type rather than picking the
      // wrong token.
      type: typIndex >= 0 ? (parts[typIndex + 1] || '') : '',
    };
  }

  /** `maxCandidates` is exposed as a parameter for tests; blip_net.js
   * itself calls this with the default. */
  function slimSdpForQr(sdp, maxCandidates) {
    var limit = maxCandidates || DEFAULT_MAX_CANDIDATES;
    var candidateCount = 0;
    var kept = sdp.split('\r\n').filter(function (line) {
      var candidate = parseCandidateLine(line);
      // Not a candidate line at all (ordinary SDP) — or one so malformed
      // parseCandidateLine couldn't even find an address/port in it —
      // pass it through untouched either way; only a *recognized*
      // candidate is ever a trimming decision.
      if (!candidate) return true;
      // Chrome/Safari hide a candidate's real LAN IP behind a random
      // `*.local` mDNS hostname by default (privacy) — that's normal and
      // must be kept; only drop literal IPv6 addresses (they contain
      // ':', an mDNS name or IPv4 dotted-quad never does) and anything
      // that isn't a plain "host" candidate (no STUN/TURN is configured
      // here, so srflx/relay shouldn't appear, but skip them too if they
      // ever do — a QR code is no place for a candidate that needs a
      // relay anyway).
      if (candidate.type !== 'host' || candidate.address.indexOf(':') !== -1) return false;
      // TCP candidates cannot connect us and only make the code denser.
      // Chrome offers `tcptype active` candidates on port 9, which can
      // only pair with a *passive* TCP candidate on the other side —
      // something neither peer ever offers, since the DataChannel runs
      // over UDP/DTLS/SCTP and there is no TURN server to provide one.
      // They are pure payload: four lines of a QR code that describe
      // routes guaranteed to fail. Dropping them also shortens the live
      // candidate list the pairing modal now shows, which would
      // otherwise fill with red rows that were never going to work.
      if (candidate.protocol && candidate.protocol.toLowerCase() !== 'udp') return false;
      if (candidateCount >= limit) return false;
      candidateCount++;
      return true;
    }).map(function (line) {
      // Strip the optional trailing extension attributes. `generation`,
      // `network-id`, `network-cost` and a per-candidate `ufrag` are all
      // optional in RFC 5245's candidate grammar — the remote parses
      // foundation, component, transport, priority, address, port and
      // `typ`, and ignores the rest. They cost ~40 bytes per candidate,
      // which on a code this size is the difference between a QR version
      // and the next one down, i.e. physically larger modules for the
      // camera to resolve.
      if (line.indexOf('a=candidate:') !== 0) return line;
      return line.replace(/ (?:generation \d+|network-id \d+|network-cost \d+|ufrag \S+)/g, '');
    });
    // Filtering to zero usable candidates would produce an SDP with no
    // route in it at all — worse than a dense QR code. Only trim when it
    // actually leaves something to connect with.
    return candidateCount > 0 ? kept.join('\r\n') : sdp;
  }

  /* ---- Compact QR payload ------------------------------------------
   *
   * A datachannel offer is ~538 bytes of SDP, of which only ~98 are
   * facts the other side cannot derive: the ICE ufrag and password, the
   * DTLS fingerprint, which DTLS role we are taking, and the host
   * candidates. Everything else — the version, origin, bundle group,
   * media line, sctp port — is boilerplate identical on both sides of
   * every pairing this app will ever do.
   *
   * Sending only the irreducible part takes the QR from 89 modules to
   * 53 while *raising* error correction from M to H. That is modules
   * 1.7x larger and double the damage tolerance at the same physical
   * size, which is the difference between a code that needs a steady
   * hand in good light and one that just scans.
   *
   * Format:  B1|ufrag|pwd|fingerprint-b64|setup|host:port,host:port
   *
   * The fingerprint travels as base64 of its 32 raw bytes rather than
   * the 95-character colon-separated hex SDP uses — same information,
   * 43 characters. `setup` is one character. Candidate addresses may be
   * mDNS `.local` names as well as IPv4 literals, so they are kept
   * verbatim.
   *
   * Anything that does not start with the `B1|` marker is treated as a
   * plain SDP and passed through untouched, so a payload this codec
   * cannot produce (or a future format) is never silently misread.
   */
  var COMPACT_PREFIX = 'B1|';
  var SETUP_CODES = { actpass: 'a', active: 'c', passive: 'p' };
  var SETUP_NAMES = { a: 'actpass', c: 'active', p: 'passive' };

  function hexFingerprintToB64(hex) {
    var bytes = hex.split(':');
    var chars = '';
    for (var i = 0; i < bytes.length; i++) {
      var v = parseInt(bytes[i], 16);
      if (isNaN(v)) return '';
      chars += String.fromCharCode(v);
    }
    return btoa(chars).replace(/=+$/, '');
  }

  function b64FingerprintToHex(b64) {
    var raw;
    try { raw = atob(b64.replace(/-/g, '+').replace(/_/g, '/')); } catch (e) { return ''; }
    var out = [];
    for (var i = 0; i < raw.length; i++) {
      var h = raw.charCodeAt(i).toString(16).toUpperCase();
      out.push(h.length === 1 ? '0' + h : h);
    }
    return out.join(':');
  }

  /** SDP -> compact string. Returns the original SDP unchanged if any
   * required field is missing, so a payload we cannot faithfully rebuild
   * is never produced. */
  function packForQr(sdp) {
    var text = String(sdp || '');
    var ufrag = (text.match(/a=ice-ufrag:(\S+)/) || [])[1];
    var pwd = (text.match(/a=ice-pwd:(\S+)/) || [])[1];
    var fp = (text.match(/a=fingerprint:sha-256 (\S+)/i) || [])[1];
    var setup = (text.match(/a=setup:(\S+)/) || [])[1];
    if (!ufrag || !pwd || !fp || !SETUP_CODES[setup]) return text;
    if (ufrag.indexOf('|') !== -1 || pwd.indexOf('|') !== -1) return text;

    var candidates = [];
    text.split(/\r\n|\n/).forEach(function (line) {
      var c = parseCandidateLine(line);
      if (!c || c.type !== 'host') return;
      if (c.protocol && c.protocol.toLowerCase() !== 'udp') return;
      if (c.address.indexOf(':') !== -1) return;   // literal IPv6
      if (c.address.indexOf('|') !== -1 || c.address.indexOf(',') !== -1) return;
      candidates.push(c.address + ':' + c.port);
    });
    if (!candidates.length) return text;

    var fpB64 = hexFingerprintToB64(fp);
    if (!fpB64) return text;
    return COMPACT_PREFIX + [ufrag, pwd, fpB64, SETUP_CODES[setup], candidates.join(',')].join('|');
  }

  /** Compact string -> SDP. Anything without the marker is returned
   * unchanged, so plain SDP still works. Returns '' for a payload that
   * claims the format but is malformed — the caller's existing SDP
   * validation then rejects it with its usual message. */
  function unpackFromQr(text) {
    var raw = String(text || '');
    if (raw.slice(0, COMPACT_PREFIX.length) !== COMPACT_PREFIX) return raw;
    var parts = raw.slice(COMPACT_PREFIX.length).split('|');
    if (parts.length !== 5) return '';
    var ufrag = parts[0], pwd = parts[1], fpB64 = parts[2];
    var setup = SETUP_NAMES[parts[3]];
    var candidates = parts[4].split(',').filter(Boolean);
    if (!ufrag || !pwd || !setup || !candidates.length) return '';
    var fp = b64FingerprintToHex(fpB64);
    if (fp.length !== 95) return ''; // 32 bytes as AA:BB:...
    var lines = [
      'v=0',
      'o=- 0 2 IN IP4 127.0.0.1',
      's=-',
      't=0 0',
      'a=group:BUNDLE 0',
      'm=application 9 UDP/DTLS/SCTP webrtc-datachannel',
      'c=IN IP4 0.0.0.0',
      'a=ice-ufrag:' + ufrag,
      'a=ice-pwd:' + pwd,
      'a=ice-options:trickle',
      'a=fingerprint:sha-256 ' + fp,
      'a=setup:' + setup,
      'a=mid:0',
      'a=sctp-port:5000',
      'a=max-message-size:262144'
    ];
    for (var i = 0; i < candidates.length; i++) {
      var at = candidates[i].lastIndexOf(':');
      if (at < 1) return '';
      var addr = candidates[i].slice(0, at);
      var port = candidates[i].slice(at + 1);
      if (!/^\d+$/.test(port)) return '';
      lines.push('a=candidate:' + (i + 1) + ' 1 udp ' + (2122260223 - i) + ' ' + addr + ' ' + port + ' typ host');
    }
    return lines.join('\r\n') + '\r\n';
  }

  var api = {
    slimSdpForQr: slimSdpForQr,
    parseCandidateLine: parseCandidateLine,
    packForQr: packForQr,
    unpackFromQr: unpackFromQr
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipSdpSlim = api;
  }
}(typeof window !== 'undefined' ? window : this));
