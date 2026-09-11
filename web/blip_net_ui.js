/* Rally-only "PLAY NEARBY" pairing UI — the modal on top of web/blip_net.js.
 * Reuses the .blip-hs-* modal classes (shell.css) that blip_scores.js's
 * name/recovery prompts already use elsewhere, so this needed no new CSS.
 * Kept separate from blip_net.js: that file is pure networking (and is
 * what test/multiplayer.mjs drives directly, headless, with no DOM UI in
 * the way); this one is just the on-screen wiring around it.
 */
(function () {
  'use strict';
  if (!window.BlipNet || !window.BlipNetProto) return; // scripts missing/out of order — fail quiet, not broken

  var proto = window.BlipNetProto;
  var modalEl = null;

  function el(tag, className, parent) {
    var e = document.createElement(tag);
    if (className) e.className = className;
    if (parent) parent.appendChild(e);
    return e;
  }

  // Dismiss the modal WITHOUT touching the connection — the success path
  // (a match just connected and is about to start) needs the DataChannel
  // to survive the modal closing, not get torn down by it.
  function dismissModal() {
    if (modalEl) { modalEl.remove(); modalEl = null; }
    if (stopActiveScan) { stopActiveScan(); stopActiveScan = null; }
  }
  // The CLOSE button / an abort: dismiss AND actually cancel — there's no
  // live connection worth keeping in either case (canceling is a no-op if
  // nothing is connected yet).
  function closeModal() {
    dismissModal();
    window.BlipNet.cancel();
  }

  function openModal() {
    if (modalEl) return;
    modalEl = el('div', 'blip-hs-modal', document.body);
    var panel = el('div', 'blip-hs-panel', modalEl);
    el('div', 'blip-hs-title', panel).textContent = 'PLAY NEARBY';
    var sub = el('div', 'blip-hs-sub', panel);
    sub.textContent = 'Both phones need to be on the same WiFi.';

    var body = el('div', '', panel);
    showChoice(body, panel);

    var row = el('div', 'blip-hs-row', panel);
    var close = el('button', 'blip-hs-btn ghost', row);
    close.type = 'button';
    close.textContent = 'CLOSE';
    close.addEventListener('click', closeModal);
  }

  function clear(node) { while (node.firstChild) node.removeChild(node.firstChild); }

  function showChoice(body, panel) {
    clear(body);
    var row = el('div', 'blip-hs-row', body);
    var hostBtn = el('button', 'blip-hs-btn', row);
    hostBtn.type = 'button';
    hostBtn.textContent = 'HOST';
    hostBtn.addEventListener('click', function () { showHosting(body, panel); });

    var joinBtn = el('button', 'blip-hs-btn', row);
    joinBtn.type = 'button';
    joinBtn.textContent = 'JOIN';
    joinBtn.addEventListener('click', function () { showJoinForm(body, panel); });

    // The WiFi/Realtime path above needs both phones to reach the internet
    // for a moment to trade the offer/answer (see docs/multiplayer.md).
    // This one needs neither WiFi nor a signal — the SDP travels as a QR
    // code between the two cameras — but costs two camera scans instead of
    // typing a 4-digit code.
    if (window.BlipQR) {
      var qrLink = el('button', 'blip-hs-btn ghost', body);
      qrLink.type = 'button';
      qrLink.style.cssText = 'margin-top:10px;width:100%;';
      qrLink.textContent = 'NO WIFI? PAIR VIA QR CODE';
      qrLink.addEventListener('click', function () { showQrChoice(body, panel); });
    }
  }

  function showQrChoice(body, panel) {
    clear(body);
    el('div', 'blip-hs-sub', body).textContent =
      'No network needed — just point each phone’s camera at the other’s screen when asked.';
    var row = el('div', 'blip-hs-row', body);
    var hostBtn = el('button', 'blip-hs-btn', row);
    hostBtn.type = 'button';
    hostBtn.textContent = 'HOST';
    hostBtn.addEventListener('click', function () { showHostQR(body, panel); });

    var joinBtn = el('button', 'blip-hs-btn', row);
    joinBtn.type = 'button';
    joinBtn.textContent = 'JOIN';
    joinBtn.addEventListener('click', function () { showJoinQR(body, panel); });

    var back = el('button', 'blip-hs-btn ghost', body);
    back.type = 'button';
    back.style.cssText = 'margin-top:10px;width:100%;';
    back.textContent = 'BACK';
    back.addEventListener('click', function () { showChoice(body, panel); });
  }

  function statusText(s, detail) {
    switch (s) {
      case 'waiting':  return detail ? 'Waiting for your friend to join…' : 'Waiting to connect…';
      case 'answering': return 'Connecting…';
      case 'connected': return 'Connected! Starting…';
      case 'disconnected': return 'Disconnected.';
      case 'failed': return 'Could not connect. Make sure you’re on the same network and try again.';
      case 'timeout': return 'Nobody joined in time.';
      case 'bad_code': return 'That code doesn’t look right — check the 4 digits.';
      case 'unavailable': return 'Nearby play isn’t available right now.';
      default: return '';
    }
  }

  function showHosting(body, panel) {
    clear(body);
    var codeBox = el('div', 'blip-hs-code', body);
    codeBox.textContent = '…';
    var msg = el('div', 'blip-hs-sub', body);

    var code = window.BlipNet.host(function (s, detail) {
      msg.textContent = statusText(s, detail);
      if (s === 'connected') setTimeout(dismissModal, 600);
      if (s === 'failed' || s === 'timeout') {
        setTimeout(function () { if (modalEl) showChoice(body, panel); }, 1500);
      }
    });
    codeBox.textContent = code || '----';
    msg.textContent = 'Tell your friend to tap JOIN and enter this code.';
  }

  function showJoinForm(body, panel) {
    clear(body);
    var input = el('input', 'blip-hs-input', body);
    input.setAttribute('inputmode', 'numeric');
    input.setAttribute('maxlength', String(proto.ROOM_CODE_LEN));
    input.placeholder = '0000';
    var err = el('div', 'blip-hs-err', body);
    var row = el('div', 'blip-hs-row', body);
    var go = el('button', 'blip-hs-btn', row);
    go.type = 'button';
    go.textContent = 'CONNECT';

    input.addEventListener('input', function () {
      input.value = proto.normalizeRoomCode(input.value);
    });
    function submit() {
      var code = input.value;
      if (!proto.isValidRoomCode(code)) { err.textContent = statusText('bad_code'); return; }
      err.textContent = '';
      go.disabled = true;
      input.disabled = true;
      window.BlipNet.join(code, function (s) {
        err.textContent = statusText(s);
        if (s === 'connected') { setTimeout(dismissModal, 600); return; }
        if (s === 'failed' || s === 'timeout' || s === 'bad_code') {
          go.disabled = false;
          input.disabled = false;
        }
      });
    }
    go.addEventListener('click', submit);
    input.addEventListener('keydown', function (e) { if (e.key === 'Enter') submit(); });
  }

  // ---- QR-code pairing (no network at all — see web/blip_net.js) --------

  function showHostQR(body, panel) {
    clear(body);
    var canvas = el('canvas', '', body);
    canvas.style.cssText = 'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;'+'touch-action:auto;margin:10px auto;width:200px;height:200px;'+'image-rendering:pixelated;border-radius:4px;';
    var msg = el('div', 'blip-hs-sub', body);
    msg.textContent = 'Generating code…';

    window.BlipNet.hostQR(function (offerSdp) {
      window.BlipQR.render(canvas, offerSdp);
      msg.textContent = 'Have your friend tap JOIN → SCAN, then point their camera at this code.';
      showScanButton(body, panel, 'SCAN THEIR ANSWER', function (text) {
        window.BlipNet.submitAnswer(text);
      });
    }, function (s) {
      if (s === 'connected') { setTimeout(dismissModal, 600); return; }
      if (s === 'failed' || s === 'timeout') {
        setTimeout(function () { if (modalEl) showQrChoice(body, panel); }, 1500);
      }
    });
  }

  function showJoinQR(body, panel) {
    clear(body);
    el('div', 'blip-hs-sub', body).textContent = 'Point your camera at the host’s code.';
    showScanButton(body, panel, 'SCAN HOST’S CODE', function (offerSdp) {
      clear(body);
      var canvas = el('canvas', '', body);
      canvas.style.cssText = 'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;'+'touch-action:auto;margin:10px auto;width:200px;height:200px;'+'image-rendering:pixelated;border-radius:4px;';
      var msg = el('div', 'blip-hs-sub', body);
      msg.textContent = 'Generating your answer…';
      window.BlipNet.joinQR(offerSdp, function (answerSdp) {
        window.BlipQR.render(canvas, answerSdp);
        msg.textContent = 'Show this to your friend — have them tap SCAN THEIR ANSWER.';
      }, function (s) {
        msg.textContent = statusText(s);
        if (s === 'connected') { setTimeout(dismissModal, 600); return; }
        if (s === 'failed' || s === 'timeout') {
          setTimeout(function () { if (modalEl) showQrChoice(body, panel); }, 1500);
        }
      });
    }, true);
  }

  var stopActiveScan = null;

  /** A button that, once tapped, opens the camera and scans for one QR
   * code — `onScanned(text)` fires once, after which the camera stops
   * itself. `autoStart` skips the button and opens the camera immediately
   * (the guest's very first step has nothing else to tap first). */
  function showScanButton(body, panel, label, onScanned, autoStart) {
    var holder = el('div', '', body);
    function startScanning() {
      clear(holder);
      var video = el('video', '', holder);
      video.setAttribute('playsinline', '');
      video.setAttribute('muted', '');
      video.muted = true;
      video.style.cssText = 'display:block;position:static;margin:10px auto;width:220px;height:220px;object-fit:cover;border-radius:4px;background:#000;';
      var err = el('div', 'blip-hs-err', holder);
      stopActiveScan = window.BlipQR.scan(video, function (text, scanErr) {
        stopActiveScan = null;
        if (scanErr) { err.textContent = 'Camera unavailable — check permissions.'; return; }
        clear(holder);
        onScanned(text);
      });
    }
    if (autoStart) {
      startScanning();
    } else {
      var btn = el('button', 'blip-hs-btn', holder);
      btn.type = 'button';
      btn.textContent = label;
      btn.addEventListener('click', function () { startScanning(); }, { once: true });
    }
  }

  window.addEventListener('DOMContentLoaded', function () {
    var btn = document.getElementById('net-play-btn');
    if (btn) btn.addEventListener('click', openModal);
  });
}());
