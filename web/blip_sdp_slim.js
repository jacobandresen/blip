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
 * Also exports parseCandidateLine() — the one place `a=candidate:` line
 * syntax gets parsed, reused by web/blip_net_ui.js's printCandidates() so
 * that parsing exists (and is unit-tested) in exactly one spot instead of
 * being duplicated between here and there.
 *
 * Pure string logic, no DOM/WebRTC types touched — dual-exported (Node
 * for test/sdp-slim.test.mjs, `window.BlipSdpSlim` in the browser) the
 * same way web/blip_net_proto.js is.
 */
(function (root) {
  'use strict';

  var DEFAULT_MAX_CANDIDATES = 4;

  /** Parse one SDP line as an `a=candidate:` line — the exact fields
   * slimSdpForQr's own filtering and blip_net_ui.js's printCandidates()
   * both need out of it:
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
      if (candidateCount >= limit) return false;
      candidateCount++;
      return true;
    });
    // Filtering to zero usable candidates would produce an SDP with no
    // route in it at all — worse than a dense QR code. Only trim when it
    // actually leaves something to connect with.
    return candidateCount > 0 ? kept.join('\r\n') : sdp;
  }

  var api = { slimSdpForQr: slimSdpForQr, parseCandidateLine: parseCandidateLine };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipSdpSlim = api;
  }
}(typeof window !== 'undefined' ? window : this));
