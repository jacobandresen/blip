/* Pure wire-format logic for two-device Rally (docs/multiplayer.md).
 * No DOM, no WebRTC, no Supabase — just encode/decode and validation, so
 * it can be unit-tested from plain Node (test/multiplayer-proto.test.mjs)
 * as well as loaded as a browser <script> by blip_net.js. The binary
 * host-state packet has its own format, owned by the Rust side
 * (crates/rally/src/net.rs) — this file only covers the guest -> host
 * input messages and the room-code text players type at each other.
 *
 * Dual-mode export: a plain global in a browser (`<script>` has no
 * `module`), a CommonJS export under Node (for the test file) — same
 * pattern the rest of web/ avoids needing a bundler for.
 */
(function (root) {
  'use strict';

  var PROTO_VERSION = 1;

  // ---- room codes ----------------------------------------------------
  // Short enough to read aloud / type on a phone keyboard in a hurry;
  // digits only (no ambiguous letter/number mix like the recovery codes
  // need to guard against — a room code is worthless to anyone the
  // instant the match starts, so there's no brute-force concern to design
  // around here, just typos).
  var ROOM_CODE_LEN = 4;
  var ROOM_CODE_RE = /^[0-9]{4}$/;

  /** A random 4-digit room code, zero-padded (e.g. "0042"). */
  function genRoomCode() {
    var n = Math.floor(Math.random() * 10000);
    return String(n).padStart(ROOM_CODE_LEN, '0');
  }

  /** Normalize a player's typed-in code: strip whitespace, digits only. */
  function normalizeRoomCode(raw) {
    return String(raw == null ? '' : raw).replace(/\D/g, '').slice(0, ROOM_CODE_LEN);
  }

  /** True for a well-formed 4-digit code (after normalizing). */
  function isValidRoomCode(raw) {
    return ROOM_CODE_RE.test(normalizeRoomCode(raw));
  }

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

  /** A "someone's leaving" notice — either direction, harmless if unsent. */
  function encodeBye() {
    return JSON.stringify({ v: PROTO_VERSION, t: 'bye' });
  }
  function decodeBye(raw) {
    var obj;
    try { obj = JSON.parse(raw); } catch (e) { return false; }
    return !!obj && obj.v === PROTO_VERSION && obj.t === 'bye';
  }

  var api = {
    PROTO_VERSION: PROTO_VERSION,
    ROOM_CODE_LEN: ROOM_CODE_LEN,
    genRoomCode: genRoomCode,
    normalizeRoomCode: normalizeRoomCode,
    isValidRoomCode: isValidRoomCode,
    encodeInput: encodeInput,
    decodeInput: decodeInput,
    encodeBye: encodeBye,
    decodeBye: decodeBye
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipNetProto = api;
  }
}(typeof window !== 'undefined' ? window : this));
