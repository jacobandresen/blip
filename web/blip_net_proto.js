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

  // Everything below decodes bytes that arrived from the other device.
  // Pairing is a QR code anyone in the room can photograph, there is no
  // authentication step, and a DataChannel peer can send whatever it
  // likes as fast as it likes — so these are untrusted inputs, and the
  // limits here are the boundary that keeps a malformed or hostile
  // packet from costing more than it should.

  /** Longest packet we will even attempt to parse. Real packets are
   * tiny: an input message is ~50 characters, a ping ~60. The cap is
   * checked *before* JSON.parse because parse cost scales with input
   * size and it blocks the main thread — the same thread running the
   * game loop. Without it, one peer can freeze the other's frame rate by
   * sending a multi-megabyte string, no malformed JSON required. */
  var MAX_PACKET_CHARS = 4096;

  /** Longest ping id we will accept, and therefore the longest one we
   * can be induced to echo back in a pong. Ids this module generates are
   * ~25 characters. Bounding it on the way in is what stops a peer from
   * using ping/pong as an amplifier: send one huge id, get it reflected
   * back. */
  var MAX_ID_CHARS = 64;

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
    if (typeof raw !== 'string') return null;
    if (raw.length > MAX_PACKET_CHARS) return null; // before parsing — see MAX_PACKET_CHARS
    var obj;
    try { obj = JSON.parse(raw); } catch (e) { return null; }
    // Arrays are typeof 'object' too, hence the explicit check: a packet
    // is a plain object or it is not a packet.
    if (!obj || typeof obj !== 'object' || Array.isArray(obj)) return null;
    if (obj.v !== PROTO_VERSION) return null;
    return obj;
  }

  /** An id is usable only if it is a string of sane length — the same
   * test for a ping we are about to answer and a pong we are matching
   * against one we sent. */
  function validId(id) {
    return typeof id === 'string' && id.length > 0 && id.length <= MAX_ID_CHARS;
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
    if (!obj || obj.t !== 'ping' || !validId(obj.id)) return null;
    return { id: obj.id };
  }
  function encodePong(id) {
    return JSON.stringify({ v: PROTO_VERSION, t: 'pong', id: String(id) });
  }
  function decodePong(raw) {
    var obj = parsePacket(raw);
    if (!obj || obj.t !== 'pong' || !validId(obj.id)) return null;
    return { id: obj.id };
  }

  var api = {
    PROTO_VERSION: PROTO_VERSION,
    MAX_PACKET_CHARS: MAX_PACKET_CHARS,
    MAX_ID_CHARS: MAX_ID_CHARS,
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
