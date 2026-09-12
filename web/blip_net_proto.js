/* Pure wire-format logic for two-device Rally (docs/multiplayer.md).
 * No DOM, no WebRTC — just encode/decode, so it can be unit-tested from
 * plain Node (test/multiplayer-proto.test.mjs) as well as loaded as a
 * browser <script> by blip_net.js. The binary host-state packet has its
 * own format, owned by the Rust side (crates/rally/src/net.rs) — this
 * file only covers the guest -> host input messages (the offer/answer
 * SDP that goes through QR codes is passed around as plain strings, no
 * framing needed).
 *
 * Dual-mode export: a plain global in a browser (`<script>` has no
 * `module`), a CommonJS export under Node (for the test file) — same
 * pattern the rest of web/ avoids needing a bundler for.
 */
(function (root) {
  'use strict';

  var PROTO_VERSION = 1;

  // ---- guest -> host input messages -----------------------------------
  // Plain JSON, tagged with a version from day one since this is exactly
  // the kind of thing that needs to grow a field later without breaking
  // whichever side updates first. `up`/`down` are the paddle-dial state,
  // not raw key names, so the host side only has to know "is the remote
  // paddle told to move up/down", the same shape a local key_held() reads.
  function encodeInput(up, down) {
    return JSON.stringify({ v: PROTO_VERSION, t: 'input', up: !!up, down: !!down });
  }

  /**
   * Parse an inbound input message. Returns `null` for anything that
   * isn't a well-formed `{t:'input',...}` packet of a version we
   * understand — a torn/garbled/foreign message should be dropped, never
   * thrown on (this is attacker- and packet-loss-adjacent input, treat it
   * like any other untrusted network payload).
   */
  function decodeInput(raw) {
    var obj;
    try { obj = JSON.parse(raw); } catch (e) { return null; }
    if (!obj || typeof obj !== 'object') return null;
    if (obj.v !== PROTO_VERSION) return null;
    if (obj.t !== 'input') return null;
    return { up: !!obj.up, down: !!obj.down };
  }

  var api = {
    PROTO_VERSION: PROTO_VERSION,
    encodeInput: encodeInput,
    decodeInput: decodeInput
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipNetProto = api;
  }
}(typeof window !== 'undefined' ? window : this));
