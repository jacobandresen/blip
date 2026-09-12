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

  /** Shared by every decoder below: valid JSON, an object (not an array,
   * not a primitive), of a protocol version we understand. Anything else
   * (torn/garbled/foreign data) comes back `null` rather than throwing —
   * this is attacker- and packet-loss-adjacent input, treat it like any
   * other untrusted network payload. Callers still check their own `t`. */
  function parsePacket(raw) {
    var obj;
    try { obj = JSON.parse(raw); } catch (e) { return null; }
    if (!obj || typeof obj !== 'object') return null;
    if (obj.v !== PROTO_VERSION) return null;
    return obj;
  }

  /**
   * Parse an inbound input message. Returns `null` for anything that
   * isn't a well-formed `{t:'input',...}` packet of a version we
   * understand.
   */
  function decodeInput(raw) {
    var obj = parsePacket(raw);
    if (!obj || obj.t !== 'input') return null;
    return { up: !!obj.up, down: !!obj.down };
  }

  // ---- app-level ping/pong -----------------------------------------------
  // Either side can send one, over the same DataChannel that carries
  // input/state — a round trip that actually reaches the other page's JS
  // and comes back, not just "RTCPeerConnection.connectionState says
  // connected" (that can be true for a channel whose peer has since gone
  // unresponsive, e.g. its tab was backgrounded/killed). `id` is an
  // opaque string the sender picks and the reply echoes back, so a stray
  // late pong from an earlier, already-timed-out ping can't be mistaken
  // for the current one — see web/blip_net.js's `ping()`.
  function encodePing(id) {
    return JSON.stringify({ v: PROTO_VERSION, t: 'ping', id: String(id) });
  }
  function decodePing(raw) {
    var obj = parsePacket(raw);
    if (!obj || obj.t !== 'ping' || typeof obj.id !== 'string') return null;
    return { id: obj.id };
  }
  function encodePong(id) {
    return JSON.stringify({ v: PROTO_VERSION, t: 'pong', id: String(id) });
  }
  function decodePong(raw) {
    var obj = parsePacket(raw);
    if (!obj || obj.t !== 'pong' || typeof obj.id !== 'string') return null;
    return { id: obj.id };
  }

  var api = {
    PROTO_VERSION: PROTO_VERSION,
    encodeInput: encodeInput,
    decodeInput: decodeInput,
    encodePing: encodePing,
    decodePing: decodePing,
    encodePong: encodePong,
    decodePong: decodePong
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipNetProto = api;
  }
}(typeof window !== 'undefined' ? window : this));
