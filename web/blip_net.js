/* Two-device Rally networking (docs/multiplayer.md).
 *
 * Signaling: QR codes only. The offer/answer SDP travels as a QR code
 * shown on one screen and scanned by the other's camera ("vanilla"/
 * non-trickle ICE: each side waits for its own candidate gathering to
 * finish before rendering its code, so there's no separate candidate-
 * exchange round to race against, and each side only ever needs to show
 * one code). No network of any kind is involved in pairing — this file
 * has no camera or QR-drawing code itself, that's web/blip_qr.js and
 * web/blip_net_ui.js's job; this only knows SDP strings in and out.
 *
 * Transport once connected: one unreliable/unordered RTCDataChannel,
 * both directions —
 *   guest -> host: JSON `{t:'input',...}` (blip_net_proto.js), driven by
 *     listening to the SAME KeyI/KeyK KeyboardEvents the local P2 dial
 *     already dispatches on #glcanvas (see blip_controller.js's
 *     bindDial) — this file never touches the guest's own WASM at all.
 *   host -> guest: the raw binary state packet Rally's Rust side packs
 *     (crates/rally/src/net.rs) and pushes out via the blip_net_send
 *     import every frame; forwarded here byte-for-byte, unparsed.
 *
 * Bridge (web/blip_bridge.js) calls three globals this file defines:
 *   blipNetRole()          -> 0 (none yet) | 1 (host) | 2 (guest)
 *   blipNetSend(Uint8Array) -> forward host state out over the channel
 *   blipNetPoll()           -> Uint8Array | null, latest state for a guest
 */
(function () {
  'use strict';

  var ICE_GATHER_TIMEOUT_MS = 2500;   // vanilla ICE: cap how long we wait to bundle candidates
  var CONNECT_TIMEOUT_MS = 60000;     // give up and report a clear status rather than hang forever
                                       // (generous: two camera scans take longer than typing a code)

  var proto = window.BlipNetProto;

  var pc = null;               // RTCPeerConnection
  var dc = null;               // RTCDataChannel
  var role = 0;                // 0 none, 1 host, 2 guest — what blipNetRole() reports
  var latestState = null;      // most recent inbound state packet (guest), consumed by blipNetPoll
  var connectTimer = null;
  var keyState = { KeyI: false, KeyK: false }; // guest's own tracked input, for edge-triggered dispatch
  var onStatusCb = null;

  function status(s, detail) {
    if (typeof onStatusCb === 'function') {
      try { onStatusCb(s, detail); } catch (e) { /* a listener's own bug shouldn't break signaling */ }
    }
  }

  function clearConnectTimer() {
    if (connectTimer) { clearTimeout(connectTimer); connectTimer = null; }
  }

  /** Wait for ICE gathering to finish (or time out) so the SDP we send is
   * "complete" — the vanilla-ICE trade-off documented at the top of this
   * file. Resolves either way; a timeout just means we send whatever
   * candidates showed up in time (host candidates typically appear
   * within milliseconds on a LAN, which is the only case v1 targets —
   * see docs/multiplayer.md's same-room scope decision). */
  function waitForIceGathering(peer) {
    if (peer.iceGatheringState === 'complete') return Promise.resolve();
    return new Promise(function (resolve) {
      var done = false;
      var finish = function () {
        if (done) return;
        done = true;
        peer.removeEventListener('icegatheringstatechange', onChange);
        resolve();
      };
      var onChange = function () {
        if (peer.iceGatheringState === 'complete') finish();
      };
      peer.addEventListener('icegatheringstatechange', onChange);
      setTimeout(finish, ICE_GATHER_TIMEOUT_MS);
    });
  }

  var stateLog = [];
  var STATE_LOG_MAX = 40;
  function log(tag, val) {
    stateLog.push(Date.now() + ' ' + tag + '=' + val);
    if (stateLog.length > STATE_LOG_MAX) stateLog.shift();
  }

  function newPeerConnection() {
    // No STUN/TURN: the same-room scope decision (docs/multiplayer.md)
    // means direct host candidates are always expected to suffice.
    var peer = new RTCPeerConnection({ iceServers: [] });
    peer.addEventListener('connectionstatechange', function () {
      log('connectionState', peer.connectionState);
      if (peer !== pc) return; // a stale/aborted attempt's events, ignore
      if (peer.connectionState === 'failed' || peer.connectionState === 'closed') {
        status('failed');
      }
    });
    peer.addEventListener('iceconnectionstatechange', function () {
      log('iceConnectionState', peer.iceConnectionState);
    });
    peer.addEventListener('icecandidateerror', function (e) {
      log('icecandidateerror', (e.errorText || '') + ' ' + (e.url || ''));
    });
    return peer;
  }

  function wireDataChannel(channelObj, roleValue) {
    dc = channelObj;
    dc.binaryType = 'arraybuffer';
    dc.addEventListener('open', function () {
      log('dc.open', roleValue);
      clearConnectTimer();
      role = roleValue;
      if (roleValue === 2) attachGuestInputCapture();
      status('connected');
    });
    dc.addEventListener('close', function () {
      log('dc.close', '');
      role = 0;
      latestState = null;
      detachGuestInputCapture();
      status('disconnected');
    });
    dc.addEventListener('error', function (e) {
      log('dc.error', (e.error && (e.error.message || e.error)) || '');
    });
    dc.addEventListener('message', function (e) {
      if (typeof e.data === 'string') {
        // Guest -> host input, only meaningful on the host.
        var input = proto.decodeInput(e.data);
        if (input) {
          dispatchKey('KeyI', input.up);
          dispatchKey('KeyK', input.down);
        }
        return;
      }
      // Host -> guest state, only meaningful on the guest. e.data is an
      // ArrayBuffer (binaryType set above) — hand blipNetPoll a view.
      latestState = new Uint8Array(e.data);
    });
  }

  // ---- host side: generate an offer, hand it to the UI to render as a QR ----

  function host(onOffer, onStatus) {
    onStatusCb = onStatus;
    pc = newPeerConnection();
    var channelObj = pc.createDataChannel('rally', { ordered: false, maxRetransmits: 0 });
    wireDataChannel(channelObj, 1);
    pc.createOffer()
      .then(function (offer) { return pc.setLocalDescription(offer); })
      .then(function () { return waitForIceGathering(pc); })
      .then(function () {
        onOffer(pc.localDescription.sdp);
        status('waiting');
      })
      .catch(function () { status('failed'); });

    clearConnectTimer();
    connectTimer = setTimeout(function () { status('timeout'); cancel(); }, CONNECT_TIMEOUT_MS);
  }

  /** Host: call once the guest's answer QR has been scanned. */
  function submitAnswer(sdp) {
    if (!pc) return;
    pc.setRemoteDescription({ type: 'answer', sdp: sdp }).catch(function () { status('failed'); });
  }

  // ---- guest side: `offerSdp` is whatever was scanned from the host's QR ----

  function join(offerSdp, onAnswer, onStatus) {
    onStatusCb = onStatus;
    pc = newPeerConnection();
    pc.addEventListener('datachannel', function (e) { wireDataChannel(e.channel, 2); });
    pc.setRemoteDescription({ type: 'offer', sdp: offerSdp })
      .then(function () { return pc.createAnswer(); })
      .then(function (answer) { return pc.setLocalDescription(answer); })
      .then(function () { return waitForIceGathering(pc); })
      .then(function () {
        onAnswer(pc.localDescription.sdp);
        status('answering');
      })
      .catch(function () { status('failed'); });

    clearConnectTimer();
    connectTimer = setTimeout(function () { status('timeout'); cancel(); }, CONNECT_TIMEOUT_MS);
  }

  // ---- guest's own input -> the host ---------------------------------------

  var guestKeyDownHandler = null;
  var guestKeyUpHandler = null;

  function sendGuestInput() {
    if (dc && dc.readyState === 'open') {
      dc.send(proto.encodeInput(keyState.KeyI, keyState.KeyK));
    }
  }
  function attachGuestInputCapture() {
    var canvas = document.getElementById('glcanvas');
    log('attachGuestInputCapture.canvasFound', !!canvas);
    if (!canvas) return;
    guestKeyDownHandler = function (e) {
      log('guestKeyDown', e.code);
      if (e.code === 'KeyI' || e.code === 'KeyK') {
        if (!keyState[e.code]) { keyState[e.code] = true; sendGuestInput(); }
      }
    };
    guestKeyUpHandler = function (e) {
      if (e.code === 'KeyI' || e.code === 'KeyK') {
        if (keyState[e.code]) { keyState[e.code] = false; sendGuestInput(); }
      }
    };
    canvas.addEventListener('keydown', guestKeyDownHandler);
    canvas.addEventListener('keyup', guestKeyUpHandler);
  }
  function detachGuestInputCapture() {
    var canvas = document.getElementById('glcanvas');
    if (canvas && guestKeyDownHandler) canvas.removeEventListener('keydown', guestKeyDownHandler);
    if (canvas && guestKeyUpHandler) canvas.removeEventListener('keyup', guestKeyUpHandler);
    guestKeyDownHandler = guestKeyUpHandler = null;
    keyState.KeyI = keyState.KeyK = false;
  }

  // ---- host's own dispatch of the guest's remote input -----------------------

  var lastDispatched = { KeyI: false, KeyK: false };
  function dispatchKey(code, down) {
    if (lastDispatched[code] === down) return; // edge-triggered, like a real key
    lastDispatched[code] = down;
    var canvas = document.getElementById('glcanvas');
    if (!canvas) return;
    canvas.dispatchEvent(new KeyboardEvent(down ? 'keydown' : 'keyup', {
      bubbles: true, cancelable: true, key: code === 'KeyI' ? 'i' : 'k', code: code,
    }));
  }

  function cancel() {
    clearConnectTimer();
    detachGuestInputCapture();
    if (dc) { try { dc.close(); } catch (e) {} dc = null; }
    if (pc) { try { pc.close(); } catch (e) {} pc = null; }
    role = 0;
    latestState = null;
  }

  // ---- bridge globals (see web/blip_bridge.js) ------------------------------

  window.blipNetRole = function () { return role; };
  window.blipNetSend = function (bytes) {
    if (dc && dc.readyState === 'open') dc.send(bytes);
  };
  window.blipNetPoll = function () {
    var s = latestState;
    latestState = null; // only ever hand back a packet once
    return s;
  };

  window.BlipNet = { host: host, join: join, submitAnswer: submitAnswer, cancel: cancel };
  // Debug/test introspection only — test/multiplayer.mjs and manual
  // console debugging. Not part of the public API.
  window.__blipNetDebug = function () {
    return {
      role: role,
      hasDc: !!dc,
      dcState: dc && dc.readyState,
      keyState: { KeyI: keyState.KeyI, KeyK: keyState.KeyK },
      latestStateLen: latestState ? latestState.length : 0,
      log: stateLog,
      pcConnectionState: pc && pc.connectionState,
      pcIceConnectionState: pc && pc.iceConnectionState,
    };
  };
}());
