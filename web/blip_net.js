/* Two-device Rally networking (docs/multiplayer.md).
 *
 * Signaling: QR codes only. The offer/answer SDP travels as a QR code
 * shown on one screen and scanned by the other's camera ("vanilla"/
 * non-trickle ICE: each side waits for its own candidate gathering to
 * finish before rendering its code, so there's no separate candidate-
 * exchange round to race against, and each side only ever needs to show
 * one code). No network service is involved in pairing; this file has no
 * camera or QR-drawing code itself, that's web/blip_qr.js and
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

  var ICE_GATHER_TIMEOUT_MS = 10000;  // mobile browsers can take several seconds to gather ICE
  var CONNECT_TIMEOUT_MS = 60000;     // give up and report a clear status rather than hang forever

  // A DataChannel peer is untrusted: pairing is a QR code anyone in the
  // room can photograph, and nothing authenticates the far side
  // afterwards. These bound what a peer can spend on our behalf. The
  // wire-format limits live in blip_net_proto.js; these are the ones
  // that need the connection's own context (role, timing).

  /** Largest binary frame we will keep. The state packet is 36 bytes
   * (NET_STATE_LEN in crates/rally/src/net.rs) and Rust rejects anything
   * that is not exactly that, so this is not validation — it stops a
   * peer from making us allocate a copy of an arbitrarily large buffer
   * every frame. Deliberately ~100x the real packet so a future field
   * cannot silently trip it. */
  var MAX_STATE_BYTES = 4096;

  /** Largest payload this side will ever put on the wire.
   *
   * Not a policy about peers — a guard against ourselves. Measured on
   * WebKit: sending a 1MB frame closes the sender's own DataChannel,
   * silently. `send()` returns normally, nothing throws, and the channel
   * is simply gone a moment later; 256KB goes through fine. There is no
   * catch block that can recover from that because there is no
   * exception, so the only defence is not to make the call.
   *
   * Nothing blip sends comes anywhere near this — a state packet is 36
   * bytes and an input message about 50 — so the cap costs nothing today.
   * It exists so that a future field, or a bug that hands the wrong
   * buffer to net_send, degrades into a dropped message instead of a
   * connection that dies for reasons nobody can see. */
  var MAX_SEND_BYTES = 64 * 1024;

  function payloadSize(payload) {
    if (typeof payload === 'string') return payload.length;
    if (payload && typeof payload.byteLength === 'number') return payload.byteLength;
    return 0;
  }

  /** The single way this module puts anything on the wire. */
  function safeSend(payload) {
    if (!dc || dc.readyState !== 'open') return false;
    var size = payloadSize(payload);
    if (size > MAX_SEND_BYTES) {
      log('send-too-large', size);
      return false;
    }
    try {
      dc.send(payload);
      return true;
    } catch (e) {
      log('send-failed', (e && e.message) || String(e));
      return false;
    }
  }

  /** Pongs we will emit per second. Every ping obliges a reply, which
   * makes an unbounded flood an easy way to keep the other device's main
   * thread busy. Real usage is a developer typing a diagnostic, a few
   * per minute at most. */
  /** How long ICE may sit in 'checking' before we say something.
   *
   * Pairing over a blocked network is the one failure that looks exactly
   * like success right up until it doesn't: both codes scan, both sides
   * report progress, and then nothing happens for the full connect
   * timeout. The most common cause is not a bug at all but the network —
   * guest/corporate WiFi with client isolation (the access point refuses
   * to pass traffic between its own clients), or a VPN routing LAN
   * traffic off to somewhere else. Neither is visible from inside the
   * page, and neither is something retrying will fix, so the one useful
   * thing to do is say so while the player is still watching. */
  var ICE_SLOW_HINT_MS = 8000;

  /** Reason string for "ICE ran and nothing got through". Matched by
   * blip_net_ui.js to choose a message about the network rather than a
   * generic failure — see signalText there. */
  var NET_BLOCKED_REASON = 'the devices cannot reach each other on this network';

  var iceFailed = false;
  var blockedHintTimer = null;

  /** True when this attempt is ending without ever having carried a
   * packet. Engines disagree about how a dead path is reported: Chromium
   * takes iceConnectionState to 'disconnected' and connectionState to
   * 'failed', never using ICE's own 'failed' state at all, so keying off
   * that one state alone misses the common case. What is unambiguous is
   * the combination — the DataChannel never opened (role is still 0) and
   * ICE never reached a connected state — and that is exactly the
   * blocked-network situation regardless of which state machine noticed
   * first. Guarded on role so a mid-match 'disconnected' blip, which is
   * a different thing entirely, is not mistaken for it. */
  function neverConnected(peer) {
    return role === 0 &&
      peer.iceConnectionState !== 'connected' &&
      peer.iceConnectionState !== 'completed';
  }

  function clearBlockedHint() {
    if (blockedHintTimer) {
      clearTimeout(blockedHintTimer);
      blockedHintTimer = null;
    }
  }

  function startBlockedHint(peer) {
    clearBlockedHint();
    blockedHintTimer = setTimeout(function () {
      if (peer !== pc) return;  // a superseded attempt
      if (role !== 0) return;   // already connected; nothing to warn about
      status('checking-slow');
    }, ICE_SLOW_HINT_MS);
  }

  var MAX_PONGS_PER_SEC = 10;
  var pongWindowStart = 0;
  var pongsInWindow = 0;
  // Counters exist so the limit is observable: "the channel survived a
  // flood" is true of an unlimited responder too, so a test cannot tell
  // the rate limit from its absence without them.
  var pongsSent = 0;
  var pongsDropped = 0;

  function pongBudgetAvailable() {
    var now = Date.now();
    if (now - pongWindowStart >= 1000) {
      pongWindowStart = now;
      pongsInWindow = 0;
    }
    pongsInWindow++;
    if (pongsInWindow <= MAX_PONGS_PER_SEC) {
      pongsSent++;
      return true;
    }
    pongsDropped++;
    return false;
  }
                                       // (generous: two camera scans can take a while)

  var proto = window.BlipNetProto;

  var pc = null;               // RTCPeerConnection
  var dc = null;               // RTCDataChannel
  var role = 0;                // 0 none, 1 host, 2 guest — what blipNetRole() reports
  var latestState = null;      // most recent inbound state packet (guest), consumed by blipNetPoll
  var connectTimer = null;
  var remoteSdpCandidateCount = 0;
  var keyState = { KeyI: false, KeyK: false }; // guest's own tracked input, for edge-triggered dispatch
  var onStatusCb = null;
  var pendingPings = {};        // id -> { sentAt, cb, timer } — in-flight ping() calls awaiting their pong
  var pingSeq = 0;

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

  function normalizeSdp(sdp) {
    if (typeof sdp !== 'string') return '';
    var normalized = sdp.trim().replace(/\r\n?|\n/g, '\r\n');
    return normalized ? normalized + '\r\n' : '';
  }

  function candidateCount(sdp) {
    var normalized = normalizeSdp(sdp);
    return normalized ? (normalized.match(/(?:^|\n)a=candidate:/g) || []).length : 0;
  }

  function validateSdp(sdp) {
    var normalized = normalizeSdp(sdp);
    if (!normalized) return { sdp: '', reason: 'scanned code was empty' };
    if (!/^v=0(?:\r\n|$)/.test(normalized)) return { sdp: normalized, reason: 'scanned code is not valid SDP' };
    var candidates = candidateCount(normalized);
    if (!candidates) return { sdp: normalized, reason: 'scanned SDP has no ICE candidates' };
    return { sdp: normalized, candidates: candidates };
  }

  function hasIceCandidates(sdp) {
    return candidateCount(sdp) > 0;
  }

  var stateLog = [];
  var STATE_LOG_MAX = 20;
  function log(tag, val) {
    var text = val === '' || val == null ? '' : String(val);
    if (text.length > 80) text = text.slice(0, 80);
    stateLog.push(tag + (text ? ':' + text : ''));
    if (stateLog.length > STATE_LOG_MAX) stateLog.shift();
  }

  // Candidate-trimming logic lives in web/blip_sdp_slim.js (dual Node/
  // browser export, like blip_net_proto.js) so it has real unit tests —
  // see test/sdp-slim.test.mjs. Falls back to a pass-through (no
  // slimming, but no hard crash either) if that script failed to load —
  // script tag order guarantees it normally won't, but this module
  // shouldn't go completely dark over it.
  var slimSdpForQr = (window.BlipSdpSlim && window.BlipSdpSlim.slimSdpForQr) || function (sdp) { return sdp; };
  // Compact codec (see blip_sdp_slim.js). Identity fallbacks so a missing
  // module degrades to sending plain SDP rather than breaking pairing.
  var packForQr = (window.BlipSdpSlim && window.BlipSdpSlim.packForQr) || function (sdp) { return sdp; };
  var unpackFromQr = (window.BlipSdpSlim && window.BlipSdpSlim.unpackFromQr) || function (text) { return text; };

  // WebKit can decline to produce any ICE candidate at all for a page
  // that has never been granted media-capture permission, when it has no
  // other way to offer one privately. Real iOS Safari usually does have
  // one — it emits an mDNS `<uuid>.local` host candidate, which hides the
  // LAN address instead of withholding it — but a WebKit build with no
  // mDNS responder (headless WebKit, as used by
  // test/multiplayer-webkit.mjs) emits nothing rather than expose a raw
  // IP. blip configures no ICE servers by design (pairing must work with
  // no internet, and gameplay must never be relayed), so there is no
  // STUN fallback to fall back to, and an unreachable STUN entry does not
  // help: it is the media grant WebKit gates on, not the presence of a
  // server.
  //
  // The failure is silent and total where it happens: iceGatheringState
  // never leaves 'gathering', the local description carries no
  // a=candidate lines, and host() gives up after the gather timeout.
  //
  // Taking the camera permission first costs the player nothing: both
  // pairing roles open the camera moments later anyway to scan a code,
  // so this only moves the prompt earlier. The stream is stopped the
  // instant it arrives — the permission is the point, not the video.
  // Best-effort by construction: a denial, or no mediaDevices at all
  // (iOS exposes none outside a secure context), resolves like a grant,
  // so pairing proceeds and reports its real outcome through the
  // existing "no ICE candidates" status rather than inventing a second
  // failure mode.
  var mediaWarmUp = null;
  var mediaWarmUpResult = null;
  function warmUpIceMedia() {
    if (mediaWarmUp) return mediaWarmUp;
    var md = navigator.mediaDevices;
    var remember = function (r) { mediaWarmUpResult = r; return r; };
    if (!md || !md.getUserMedia) {
      mediaWarmUp = Promise.resolve(remember('unavailable'));
      return mediaWarmUp;
    }
    mediaWarmUp = md.getUserMedia({ video: true }).then(function (stream) {
      stream.getTracks().forEach(function (t) { t.stop(); });
      return remember('granted');
    }, function (e) {
      return remember('denied:' + ((e && e.name) || 'unknown'));
    });
    return mediaWarmUp;
  }

  /** Gathering produced nothing. On WebKit that has one overwhelmingly
   * likely cause — the media permission warm-up above was refused, so
   * the engine withheld every candidate — and saying so is the
   * difference between a player who can fix it and one staring at
   * "Could not connect". Anything else (no network at all, an interface
   * with no usable address) keeps the plain reason. */
  function noCandidateReason() {
    var denied = typeof mediaWarmUpResult === 'string' &&
      (mediaWarmUpResult.indexOf('denied') === 0 || mediaWarmUpResult === 'unavailable');
    return denied
      ? 'no ICE candidates were gathered — this browser needs camera access to connect'
      : 'no ICE candidates were gathered';
  }

  /** How long the loopback probe may take before we call it a failure. */
  var PREFLIGHT_TIMEOUT_MS = 4000;

  /** Reason strings the UI matches on (see signalText in blip_net_ui.js). */
  var NO_WEBRTC_REASON = 'WebRTC could not open a connection on this device';
  var DIFFERENT_NETWORK_REASON = 'the two devices look like they are on different networks';

  var preflight = null;       // cached promise — the answer cannot change mid-session
  var localAddresses = [];    // IPv4 literals this device gathered, for the subnet check

  function candidateAddress(candidateStr) {
    // "candidate:<foundation> <component> <proto> <priority> <address> <port> typ ..."
    var parts = String(candidateStr || '').split(' ');
    return parts.length > 4 ? parts[4] : '';
  }

  function isIpv4Literal(addr) {
    return /^\d{1,3}(\.\d{1,3}){3}$/.test(addr);
  }

  function subnetOf(addr) {
    return addr.split('.').slice(0, 3).join('.'); // /24 is the useful granularity for a home LAN
  }

  /**
   * Prove the WebRTC data path works here *before* asking the player to
   * point a camera at anything.
   *
   * Two RTCPeerConnections in this very page, connected to each other,
   * with one DataChannel message actually sent and received — a probe
   * that is expected to succeed on any working setup, since the traffic
   * never leaves the machine. When it fails, the cause is local and
   * total: WebRTC disabled by enterprise policy or an extension, a
   * browser build without SCTP, or an engine that will not produce a
   * single ICE candidate (the WebKit case warmUpIceMedia() addresses).
   *
   * Running it up front turns the worst failure mode — scan a code, wait
   * a minute, get a generic error — into an immediate, accurate message.
   * It also collects this device's own addresses, which is what lets
   * join() notice the two devices are not on the same network.
   *
   * Cached: the answer cannot change within a page load, and the probe
   * is not free.
   */
  function preflightWebRtc() {
    if (preflight) return preflight;
    preflight = new Promise(function (resolve) {
      if (typeof RTCPeerConnection !== 'function') {
        resolve({ ok: false, reason: NO_WEBRTC_REASON, detail: 'no RTCPeerConnection' });
        return;
      }
      var a = null, b = null, done = false, timer = null;
      var addresses = [];

      function finish(result) {
        if (done) return;
        done = true;
        clearTimeout(timer);
        localAddresses = addresses;
        // Close both: this probe must not leave sockets or a live
        // connection behind for the real attempt to trip over.
        try { if (a) a.close(); } catch (e) {}
        try { if (b) b.close(); } catch (e) {}
        resolve(result);
      }

      try {
        a = new RTCPeerConnection({ iceServers: [] });
        b = new RTCPeerConnection({ iceServers: [] });
      } catch (e) {
        finish({ ok: false, reason: NO_WEBRTC_REASON, detail: 'construction threw: ' + ((e && e.message) || e) });
        return;
      }

      timer = setTimeout(function () {
        finish({ ok: false, reason: NO_WEBRTC_REASON, detail: 'loopback probe timed out', addresses: addresses });
      }, PREFLIGHT_TIMEOUT_MS);

      a.addEventListener('icecandidate', function (e) {
        if (!e.candidate) return;
        var addr = candidateAddress(e.candidate.candidate);
        if (isIpv4Literal(addr) && addresses.indexOf(addr) === -1) addresses.push(addr);
        b.addIceCandidate(e.candidate).catch(function () { /* the probe is best-effort */ });
      });
      b.addEventListener('icecandidate', function (e) {
        if (e.candidate) a.addIceCandidate(e.candidate).catch(function () {});
      });
      b.addEventListener('datachannel', function (e) {
        e.channel.addEventListener('message', function (ev) {
          finish({ ok: ev.data === 'blip-probe', reason: null, addresses: addresses });
        });
      });

      var probe = a.createDataChannel('blip-probe');
      probe.addEventListener('open', function () {
        try { probe.send('blip-probe'); } catch (e) {
          finish({ ok: false, reason: NO_WEBRTC_REASON, detail: 'probe send failed' });
        }
      });

      a.createOffer()
        .then(function (o) { return a.setLocalDescription(o); })
        .then(function () { return b.setRemoteDescription(a.localDescription); })
        .then(function () { return b.createAnswer(); })
        .then(function (ans) { return b.setLocalDescription(ans); })
        .then(function () { return a.setRemoteDescription(b.localDescription); })
        .catch(function (e) {
          finish({ ok: false, reason: NO_WEBRTC_REASON, detail: 'negotiation failed: ' + ((e && e.message) || e) });
        });
    });
    return preflight;
  }

  /**
   * Do the scanned code's candidates look like they are even reachable
   * from here?
   *
   * Reported as a warning rather than a failure, deliberately. Two
   * devices on different /24s can still route to each other, so treating
   * a mismatch as fatal would break legitimate setups. But the common
   * real case — one device on the WiFi, the other on a guest network, a
   * hotspot, or a VPN — is otherwise indistinguishable from "it is just
   * taking a while", right up until the connect timeout.
   *
   * Only claims a mismatch when both sides published plain IPv4 host
   * candidates; mDNS (.local) candidates carry no address to compare, so
   * the check stays quiet rather than guessing.
   */
  function looksLikeDifferentNetwork(remoteSdp) {
    if (!localAddresses.length) return false;
    var remote = [];
    String(remoteSdp || '').split(/\r\n|\n/).forEach(function (line) {
      if (line.indexOf('a=candidate:') !== 0) return;
      var addr = candidateAddress(line.slice('a='.length));
      if (isIpv4Literal(addr)) remote.push(addr);
    });
    if (!remote.length) return false;
    var mine = {};
    localAddresses.forEach(function (a) { mine[subnetOf(a)] = true; });
    for (var i = 0; i < remote.length; i++) {
      if (mine[subnetOf(remote[i])]) return false; // a shared subnet — plausible
    }
    return true;
  }

  /** Run the probe alongside the real attempt and report a local
   * WebRTC failure the moment it is known, rather than letting the
   * player scan a code that was never going to work. Guarded on `peer`
   * so a superseded attempt stays silent. */
  function reportPreflight(peer) {
    preflightWebRtc().then(function (r) {
      if (peer !== pc) return;
      log('preflight', r.ok ? ('ok ' + (r.addresses || []).join(',')) : (r.detail || 'failed'));
      if (!r.ok) {
        status('failed', { reason: NO_WEBRTC_REASON });
        cancel();
      }
    });
  }

  function newPeerConnection() {
    // Local host candidates only: pairing must work without internet and
    // gameplay must never be relayed through a server.
    var peer = new RTCPeerConnection({ iceServers: [] });
    peer.addEventListener('connectionstatechange', function () {
      log('connectionState', peer.connectionState);
      if (peer !== pc) return; // a stale/aborted attempt's events, ignore
      if (peer.connectionState === 'failed' || peer.connectionState === 'closed') {
        // Same underlying cause when ICE already gave up — pass the
        // reason on so this doesn't overwrite a specific message with a
        // generic one.
        status('failed', (iceFailed || neverConnected(peer)) ? { reason: NET_BLOCKED_REASON } : undefined);
      }
    });
    peer.addEventListener('iceconnectionstatechange', function () {
      log('iceConnectionState', peer.iceConnectionState);
      if (peer !== pc) return; // a stale/aborted attempt's events, ignore
      var st = peer.iceConnectionState;
      if (st === 'checking') {
        startBlockedHint(peer);
      } else if (st === 'connected' || st === 'completed') {
        clearBlockedHint();
      } else if (st === 'failed' || (st === 'disconnected' && role === 0)) {
        // ICE tried every candidate pair and none of them got a packet
        // through. The descriptions were exchanged fine (they came off a
        // QR code, not the network), so this is the network itself.
        // 'disconnected' counts here only before anything connected —
        // see neverConnected() on why both states have to be handled.
        clearBlockedHint();
        iceFailed = true;
        status('failed', { reason: NET_BLOCKED_REASON });
      }
    });
    peer.addEventListener('icecandidateerror', function (e) {
      log('ice-error', e.errorText || 'unknown');
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
        // App-level ping/pong (see ping() below) — either side can send
        // one, so check for both regardless of role. Checked before
        // decodeInput since they're disjoint `t` values on the same wire.
        var ping = proto.decodePing(e.data);
        if (ping) {
          // Dropped rather than queued once the budget is spent: a pong
          // is only useful if it is prompt, so a late one has no value
          // to a legitimate pinger and a backlog is exactly what a
          // flooder wants us to build.
          if (!pongBudgetAvailable()) return;
          safeSend(proto.encodePong(ping.id)); // best-effort reply
          return;
        }
        var pong = proto.decodePong(e.data);
        if (pong) {
          var waiting = pendingPings[pong.id];
          if (waiting) { // no entry: a late reply to a ping we already gave up on — ignore it
            delete pendingPings[pong.id];
            clearTimeout(waiting.timer);
            waiting.cb(Date.now() - waiting.sentAt, null);
          }
          return;
        }
        // Guest -> host input. Acted on *only* by the host: this
        // synthesises real keydown/keyup events into the page, so
        // honouring it in either direction would let whichever peer felt
        // like it drive the other player's paddle. The direction of this
        // message is part of the protocol, not a convention.
        var input = proto.decodeInput(e.data);
        if (input && roleValue === 1) {
          dispatchKey('KeyI', input.up);
          dispatchKey('KeyK', input.down);
        }
        return;
      }
      // Host -> guest state. Mirror of the input rule above: only the
      // guest renders someone else's simulation, so only the guest keeps
      // this. On the host the field is never read, and storing it would
      // just be a buffer a peer can grow at will. e.data is an
      // ArrayBuffer (binaryType set above) — hand blipNetPoll a view.
      if (roleValue !== 2) return;
      // Type-checked, not just size-checked. `binaryType` is set to
      // 'arraybuffer' above, but that is a request about how *we* decode
      // frames, not a guarantee about what arrives: a Blob (or anything
      // else) has no numeric byteLength, `undefined > MAX_STATE_BYTES`
      // is false, and the size check alone would wave it through into a
      // Uint8Array that silently comes out empty. An empty state packet
      // is worse than no packet - it is the wrong length, so Rust drops
      // it, and the guest's view freezes with no error anywhere.
      if (!(e.data instanceof ArrayBuffer)) return;
      if (e.data.byteLength === 0 || e.data.byteLength > MAX_STATE_BYTES) return;
      latestState = new Uint8Array(e.data);
    });
  }

  // ---- host side: generate an offer, hand it to the UI to render as a QR ----

  function host(onOffer, onStatus) {
    // A caller that starts a fresh host()/join() without ever cancelling
    // the previous attempt would otherwise leak the old
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
    reportPreflight(peer); // earliest possible word on a local WebRTC fault
    var channelObj = peer.createDataChannel('rally', { ordered: false, maxRetransmits: 0 });
    wireDataChannel(channelObj, 1);
    // Started before the promise chain, so it still runs inside the
    // transient user activation from the HOST tap — iOS Safari rejects a
    // getUserMedia() that has drifted out of the gesture. See
    // warmUpIceMedia() for why hosting needs it at all.
    warmUpIceMedia()
      .then(function () { return peer.createOffer(); })
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
        var sdp = peer.localDescription && peer.localDescription.sdp;
        if (!hasIceCandidates(sdp)) {
          status('failed', { reason: noCandidateReason() });
          cancel();
          return;
        }
        var qrSdp = slimSdpForQr(sdp);
        var qrCheck = validateSdp(qrSdp);
        if (qrCheck.reason) {
          status('failed', { reason: 'offer became invalid after QR compaction: ' + qrCheck.reason });
          cancel();
          return;
        }
        log('offerCandidates', qrCheck.candidates);
        onOffer(packForQr(qrCheck.sdp));
        status('waiting');
      })
      .catch(function (e) {
        if (peer !== pc) return;
        var reason = e && (e.message || String(e)) || 'offer setup failed';
        log('offer-error', reason);
        status('failed', { reason: reason });
      });

    connectTimer = setTimeout(function () {
      if (peer !== pc) return;
      // A timeout with ICE still unfinished is the blocked-network case
      // arriving the slow way: nothing failed loudly, nothing connected.
      status('timeout', peer.iceConnectionState !== 'connected' &&
        peer.iceConnectionState !== 'completed' ? { reason: NET_BLOCKED_REASON } : undefined);
      cancel();
    }, CONNECT_TIMEOUT_MS);
  }

  /** Host: call once the guest's answer QR has been scanned. */
  function submitAnswer(sdp) {
    var peer = pc;
    if (!peer) return;
    var checked = validateSdp(unpackFromQr(sdp));
    log('remoteAnswerCandidates', checked.candidates || 0);
    if (checked.reason) {
      status('failed', { reason: checked.reason });
      cancel();
      return;
    }
    remoteSdpCandidateCount = checked.candidates;
    peer.setRemoteDescription({ type: 'answer', sdp: checked.sdp }).catch(function () {
      if (peer !== pc) return; // superseded/cancelled while this was in flight
      status('failed');
    });
  }

  // ---- guest side: `offerSdp` is whatever was scanned from the host's QR ----

  function join(offerSdp, onAnswer, onStatus) {
    cancel(); // see host()'s comment on why this is unconditional
    onStatusCb = onStatus;
    var checkedOffer = validateSdp(unpackFromQr(offerSdp));
    log('remoteOfferCandidates', checkedOffer.candidates || 0);
    if (checkedOffer.reason) {
      status('failed', { reason: checkedOffer.reason });
      return;
    }
    remoteSdpCandidateCount = checkedOffer.candidates;
    var peer = newPeerConnection();
    pc = peer;
    reportPreflight(peer);
    // The guest is the first side holding *both* sets of candidates, so
    // it is the only one that can notice the two devices are not on the
    // same network — and it can say so immediately, instead of after the
    // connect timeout.
    preflightWebRtc().then(function () {
      if (peer !== pc) return;
      if (looksLikeDifferentNetwork(checkedOffer.sdp)) {
        log('network-mismatch', localAddresses.join(','));
        status('network-mismatch', { reason: DIFFERENT_NETWORK_REASON });
      }
    });
    peer.addEventListener('datachannel', function (e) {
      if (peer !== pc) return;
      wireDataChannel(e.channel, 2);
    });
    // Same WebKit ICE gate as host() — see warmUpIceMedia(). The guest
    // reaches here from the scanner's own decode callback, and
    // blip_qr.js stops the camera *before* invoking it, so there is no
    // live stream to preempt. In the real flow the permission the
    // scanner already obtained makes this resolve instantly without a
    // second prompt; it matters when join() is reached some other way
    // (a test's injected scan, or a future paste-the-code path), where
    // no camera was ever opened and WebKit would otherwise gather
    // nothing at all.
    warmUpIceMedia()
      .then(function () { return peer.setRemoteDescription({ type: 'offer', sdp: checkedOffer.sdp }); })
      .then(function () { return peer.createAnswer(); })
      .then(function (answer) { return peer.setLocalDescription(answer); })
      .then(function () { return waitForIceGathering(peer); })
      .then(function () {
        if (peer !== pc) return; // see host()'s matching comment
        var sdp = peer.localDescription && peer.localDescription.sdp;
        if (!hasIceCandidates(sdp)) {
          status('failed', { reason: noCandidateReason() });
          cancel();
          return;
        }
        var qrSdp = slimSdpForQr(sdp);
        var qrCheck = validateSdp(qrSdp);
        if (qrCheck.reason) {
          status('failed', { reason: 'answer became invalid after QR compaction: ' + qrCheck.reason });
          cancel();
          return;
        }
        log('answerCandidates', qrCheck.candidates);
        onAnswer(packForQr(qrCheck.sdp));
        status('answering');
      })
      .catch(function (e) {
        if (peer !== pc) return;
        var reason = e && (e.message || String(e)) || 'answer setup failed';
        log('answer-error', reason);
        status('failed', { reason: reason });
      });

    connectTimer = setTimeout(function () {
      if (peer !== pc) return;
      // A timeout with ICE still unfinished is the blocked-network case
      // arriving the slow way: nothing failed loudly, nothing connected.
      status('timeout', peer.iceConnectionState !== 'connected' &&
        peer.iceConnectionState !== 'completed' ? { reason: NET_BLOCKED_REASON } : undefined);
      cancel();
    }, CONNECT_TIMEOUT_MS);
  }

  // ---- guest's own input -> the host ---------------------------------------

  var guestKeyDownHandler = null;
  var guestKeyUpHandler = null;

  function sendGuestInput() {
    if (dc && dc.readyState === 'open') {
      safeSend(proto.encodeInput(keyState.KeyI, keyState.KeyK));
    }
  }
  function attachGuestInputCapture() {
    var canvas = document.getElementById('glcanvas');
    if (!canvas) return;
    guestKeyDownHandler = function (e) {
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

  /** A round trip over the already-open DataChannel — proof the other
   * page is receiving and replying, not just that WebRTC says
   * 'connected'. Kept as a small diagnostic hook for tests and tools.
   * `onResult(rttMs, errorReason)`: exactly one fires, `errorReason` is
   * `null` only on success. */
  function ping(onResult, timeoutMs) {
    if (!dc || dc.readyState !== 'open') { onResult(null, 'not connected'); return; }
    var id = 'p' + (++pingSeq) + '-' + Date.now();
    var timer = setTimeout(function () {
      if (!pendingPings[id]) return;
      delete pendingPings[id];
      onResult(null, 'timeout');
    }, timeoutMs || 3000);
    pendingPings[id] = { sentAt: Date.now(), cb: onResult, timer: timer };
    if (!safeSend(proto.encodePing(id))) {
      clearTimeout(timer);
      delete pendingPings[id];
      onResult(null, 'send failed');
    }
  }

  function clearPendingPings() {
    Object.keys(pendingPings).forEach(function (id) {
      var p = pendingPings[id];
      clearTimeout(p.timer);
      delete pendingPings[id];
      p.cb(null, 'cancelled');
    });
  }

  function cancel() {
    clearConnectTimer();
    clearBlockedHint();
    iceFailed = false;
    detachGuestInputCapture();
    clearPendingPings();
    if (dc) { try { dc.close(); } catch (e) {} dc = null; }
    if (pc) { try { pc.close(); } catch (e) {} pc = null; }
    role = 0;
    latestState = null;
    remoteSdpCandidateCount = 0;
  }

  // ---- bridge globals (see web/blip_bridge.js) ------------------------------

  window.blipNetRole = function () { return role; };
  window.blipNetSend = function (bytes) {
    safeSend(bytes);
  };
  window.blipNetPoll = function () {
    var s = latestState;
    latestState = null; // only ever hand back a packet once
    return s;
  };

  window.BlipNet = {
    host: host, join: join, submitAnswer: submitAnswer, cancel: cancel, ping: ping,
    preflight: preflightWebRtc,
    // Public because the pairing modal shows the candidate pairs while
    // it connects — see the ICE list in blip_net_ui.js. Same data the
    // __blipNetStats debug hook returns.
    iceStats: function () { return window.__blipNetStats(); },
    CONNECT_TIMEOUT_MS: CONNECT_TIMEOUT_MS,
  };
  // Test-only: send an arbitrary payload down the open DataChannel,
  // bypassing the encoders. The direction rules above (a host ignoring
  // inbound state, a guest ignoring inbound input) cannot otherwise be
  // tested from the outside, because every legitimate send path already
  // obeys them — proving they hold needs a way to send something a
  // well-behaved peer never would. Inert for players: nothing in the UI
  // calls it, and it does nothing without an open channel.
  window.__blipNetSendRaw = function (payload) {
    if (!dc || dc.readyState !== 'open') return false;
    try { dc.send(payload); return true; } catch (e) { return false; }
  };
  // Debug/test introspection only — test/multiplayer.mjs and manual
  // console debugging. Not part of the public API.
  window.__blipNetDebug = function () {
    return {
      role: role,
      hasDc: !!dc,
      dcState: dc && dc.readyState,
      keyState: { KeyI: keyState.KeyI, KeyK: keyState.KeyK },
      pongsSent: pongsSent,
      pongsDropped: pongsDropped,
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
      var localCandidates = [];
      var remoteCandidates = [];
      report.forEach(function (s) {
        if (s.type === 'local-candidate' || s.type === 'remote-candidate') {
          candidates[s.id] = s;
          (s.type === 'local-candidate' ? localCandidates : remoteCandidates).push(s);
        }
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
      function types(list) {
        var found = {};
        list.forEach(function (c) { if (c.candidateType) found[c.candidateType] = true; });
        return Object.keys(found);
      }
      return {
        localCandidateCount: localCandidates.length,
        remoteCandidateCount: remoteCandidates.length,
        localCandidateTypes: types(localCandidates),
        remoteCandidateTypes: types(remoteCandidates),
        remoteSdpCandidateCount: remoteSdpCandidateCount,
        pairs: pairs,
        connectionState: pc && pc.connectionState,
        iceConnectionState: pc && pc.iceConnectionState,
        dataChannelState: dc && dc.readyState,
      };
    }).catch(function () { return null; });
  };
}());
