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

  // Candidate-trimming logic lives in web/blip_sdp_slim.js (dual Node/
  // browser export, like blip_net_proto.js) so it has real unit tests —
  // see test/sdp-slim.test.mjs. Falls back to a pass-through (no
  // slimming, but no hard crash either) if that script failed to load —
  // script tag order guarantees it normally won't, but this module
  // shouldn't go completely dark over it.
  var slimSdpForQr = (window.BlipSdpSlim && window.BlipSdpSlim.slimSdpForQr) || function (sdp) { return sdp; };

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
    // Every handler below checks `channelObj !== dc` first: a delayed
    // event from a channel that a newer host()/join()/cancel() has since
    // superseded (`dc` now points elsewhere, or is null) must not mutate
    // shared state (`role`, `latestState`) or report status for
    // whatever attempt *is* current — the same staleness host()/join()
    // guard against on their own promise chains, just for events instead
    // of promises.
    dc.addEventListener('open', function () {
      if (channelObj !== dc) return;
      log('dc.open', roleValue);
      clearConnectTimer();
      role = roleValue;
      if (roleValue === 2) attachGuestInputCapture();
      status('connected');
    });
    dc.addEventListener('close', function () {
      if (channelObj !== dc) return;
      log('dc.close', '');
      role = 0;
      latestState = null;
      detachGuestInputCapture();
      status('disconnected');
    });
    dc.addEventListener('error', function (e) {
      if (channelObj !== dc) return;
      log('dc.error', (e.error && (e.error.message || e.error)) || '');
    });
    dc.addEventListener('message', function (e) {
      if (channelObj !== dc) return;
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
    // A caller that starts a fresh host()/join() without ever cancelling
    // the previous attempt (the JACK IN console's `host`/`join` commands
    // can do exactly this, back to back) would otherwise leak the old
    // RTCPeerConnection: its `pc`/`dc` never get replaced by anything
    // that closes them, just overwritten below, and its own event
    // listeners and in-flight promises keep running against a `pc`
    // variable that now points somewhere else entirely (see the `peer
    // !== pc` guards below — this is the same class of staleness, just
    // upstream of it). Cancelling unconditionally here makes host()/
    // join() safe to call at any time, regardless of what a caller did
    // or didn't clean up first.
    cancel();
    onStatusCb = onStatus;
    var peer = newPeerConnection();
    pc = peer;
    var channelObj = peer.createDataChannel('rally', { ordered: false, maxRetransmits: 0 });
    wireDataChannel(channelObj, 1);
    peer.createOffer()
      .then(function (offer) { return peer.setLocalDescription(offer); })
      .then(function () { return waitForIceGathering(peer); })
      .then(function () {
        // This specific attempt may have been superseded (a newer
        // host()/join() call, or an explicit cancel()) while the promise
        // chain above was still in flight — `pc` would then point at a
        // different attempt entirely (or be null). Reporting success/
        // failure for an attempt nobody is listening for any more would
        // just misattribute it to whatever *is* current.
        if (peer !== pc) return;
        onOffer(slimSdpForQr(peer.localDescription.sdp));
        status('waiting');
      })
      .catch(function () {
        if (peer !== pc) return;
        status('failed');
      });

    connectTimer = setTimeout(function () {
      if (peer !== pc) return;
      status('timeout');
      cancel();
    }, CONNECT_TIMEOUT_MS);
  }

  /** Host: call once the guest's answer QR has been scanned. */
  function submitAnswer(sdp) {
    var peer = pc;
    if (!peer) return;
    peer.setRemoteDescription({ type: 'answer', sdp: sdp }).catch(function () {
      if (peer !== pc) return; // superseded/cancelled while this was in flight
      status('failed');
    });
  }

  // ---- guest side: `offerSdp` is whatever was scanned from the host's QR ----

  function join(offerSdp, onAnswer, onStatus) {
    cancel(); // see host()'s comment on why this is unconditional
    onStatusCb = onStatus;
    var peer = newPeerConnection();
    pc = peer;
    peer.addEventListener('datachannel', function (e) {
      if (peer !== pc) return;
      wireDataChannel(e.channel, 2);
    });
    peer.setRemoteDescription({ type: 'offer', sdp: offerSdp })
      .then(function () { return peer.createAnswer(); })
      .then(function (answer) { return peer.setLocalDescription(answer); })
      .then(function () { return waitForIceGathering(peer); })
      .then(function () {
        if (peer !== pc) return; // see host()'s matching comment
        onAnswer(slimSdpForQr(peer.localDescription.sdp));
        status('answering');
      })
      .catch(function () {
        if (peer !== pc) return;
        status('failed');
      });

    connectTimer = setTimeout(function () {
      if (peer !== pc) return;
      status('timeout');
      cancel();
    }, CONNECT_TIMEOUT_MS);
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

  window.BlipNet = {
    host: host, join: join, submitAnswer: submitAnswer, cancel: cancel,
    CONNECT_TIMEOUT_MS: CONNECT_TIMEOUT_MS, // blip_net_ui.js's terminal needs this to show a countdown
  };
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
  // A "timeout" (or a long, quiet "checking") is otherwise a black box —
  // pc.iceConnectionState alone says *that* candidates are being checked,
  // not what they are or why none of them are succeeding (same-WiFi
  // "client/AP isolation" — a router refusing to let its own clients talk
  // directly to each other — is the single most common real-world cause;
  // see docs/multiplayer.md). RTCPeerConnection.getStats() has the actual
  // local/remote candidate-pair attempts and their outcome. Async (hence
  // separate from the synchronous __blipNetDebug() above) and
  // best-effort: resolves `null` rather than throwing if getStats() itself
  // isn't available or the connection has already closed.
  window.__blipNetStats = function () {
    if (!pc || typeof pc.getStats !== 'function') return Promise.resolve(null);
    return pc.getStats().then(function (report) {
      var candidates = {};
      report.forEach(function (s) {
        if (s.type === 'local-candidate' || s.type === 'remote-candidate') candidates[s.id] = s;
      });
      function describe(c) {
        if (!c) return '?';
        return [c.candidateType, c.protocol, (c.address || c.ip || '?') + ':' + (c.port != null ? c.port : '?')]
          .filter(Boolean).join(' ');
      }
      var pairs = [];
      report.forEach(function (s) {
        if (s.type !== 'candidate-pair') return;
        pairs.push({
          state: s.state,
          nominated: !!s.nominated,
          local: describe(candidates[s.localCandidateId]),
          remote: describe(candidates[s.remoteCandidateId]),
          // Whether a connectivity check actually got a response is the
          // difference between "still trying" and "something is dropping
          // this traffic" — not populated on every pair (e.g. one still
          // 'waiting' has sent nothing yet), so default to 0 rather than
          // leaving it undefined for the UI's arithmetic/display.
          requestsSent: s.requestsSent || 0,
          responsesReceived: s.responsesReceived || 0,
        });
      });
      return pairs;
    }).catch(function () { return null; });
  };
}());
