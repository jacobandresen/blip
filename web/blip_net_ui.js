/* Rally-only "PLAY NEARBY" pairing UI — the modal on top of web/blip_net.js.
 * QR-code pairing only (see docs/multiplayer.md) — no network involved in
 * pairing at all, just two camera scans. Reuses the .blip-hs-* modal
 * classes (shell.css) that blip_scores.js's name/recovery prompts already
 * use elsewhere, so this needed no new CSS beyond the QR canvas/video
 * sizing below. Kept separate from blip_net.js: that file is pure
 * networking (and is what test/multiplayer.mjs drives directly, headless,
 * with no DOM UI in the way); this one is just the on-screen wiring
 * around it.
 */
(function () {
  'use strict';
  // Any of these missing (script blocked, load order broken, one of the
  // two vendored QR libraries failed) — drop the whole feature rather
  // than show a HOST/JOIN that can't actually pair.
  if (!window.BlipNet || !window.BlipNetProto || !window.BlipQR) return;

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
    sub.textContent = 'No WiFi needed to pair — just point each phone’s camera at the other’s screen when asked. (The phones do still need to be able to reach each other once paired — usually: same WiFi.)';

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
    hostBtn.addEventListener('click', function () { showHostQR(body, panel); });

    var joinBtn = el('button', 'blip-hs-btn', row);
    joinBtn.type = 'button';
    joinBtn.textContent = 'JOIN';
    joinBtn.addEventListener('click', function () { showJoinQR(body, panel); });
  }

  function statusText(s) {
    switch (s) {
      case 'waiting':  return 'Waiting for your friend to scan…';
      case 'answering': return 'Connecting…';
      case 'connected': return 'Connected! Starting…';
      case 'disconnected': return 'Disconnected.';
      case 'failed': return 'Could not connect. Make sure both phones can reach each other and try again.';
      case 'timeout': return 'Nobody scanned in time.';
      default: return '';
    }
  }

  function qrCanvas(parent) {
    // Classed (not just typed) so test/multiplayer.mjs and the scan
    // preview's own overlay canvas below can never be confused for one
    // another — both are `<canvas>` elements inside the same panel.
    var canvas = el('canvas', 'blip-qr-canvas', parent);
    // Neutralize shell.css's bare `canvas { position:fixed; clip-path:...
    // }` rule — meant only for the game's own #glcanvas, but a bare tag
    // selector catches every canvas on the page, including this one.
    canvas.style.cssText =
      'display:block;position:static;top:auto;left:auto;transform:none;clip-path:none;' +
      'touch-action:auto;margin:10px auto;width:240px;height:240px;' +
      'image-rendering:pixelated;border-radius:4px;';
    return canvas;
  }

  /** Turn window.BlipQR's scan status callback into a human-readable line —
   * "write feedback when something happens" instead of silence while
   * nothing has decoded yet. */
  function scanStatusText(state, detail) {
    switch (state) {
      case 'opening': return 'Opening camera…';
      case 'streaming':
        return 'Camera on' + (detail.width ? ' (' + detail.width + '×' + detail.height + ')' : '') + ' — point it at the code.';
      case 'buffering': return 'Camera opened, waiting for its first frame…';
      case 'scanning': {
        var bits = [detail.frames + ' frame' + (detail.frames === 1 ? '' : 's') + ' scanned'];
        bits.push(detail.candidates > 0
          ? detail.candidates + ' possible code' + (detail.candidates === 1 ? '' : 's') + ' in view (circled below)'
          : 'no code found yet');
        if (detail.decodeError) bits.push('decode hiccup: ' + detail.decodeError);
        return bits.join(' · ');
      }
      case 'found': return 'Got it!';
      case 'play-error':
        return 'Camera opened but would not start playing' + (detail.message ? ' (' + detail.message + ')' : '') + '.';
      case 'unsupported': return 'This browser has no camera API available.';
      default: return '';
    }
  }

  /** Turn a getUserMedia() rejection into a specific, actionable message
   * instead of the old one-size-fits-all "Camera unavailable". */
  function scanErrorMessage(err) {
    var name = err && err.name;
    if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
      return 'Camera permission denied — allow camera access for this site (check the address bar / browser settings) and try again.';
    }
    if (name === 'NotFoundError' || name === 'DevicesNotFoundError') return 'No camera found on this device.';
    if (name === 'NotReadableError') return 'Camera is busy or unavailable — another app may be using it.';
    if (name === 'OverconstrainedError') return 'No camera on this device matched what was requested.';
    return 'Camera unavailable' + (err && err.message ? ': ' + err.message : '') + '.';
  }

  function showHostQR(body, panel) {
    clear(body);
    var canvas = qrCanvas(body);
    var msg = el('div', 'blip-hs-sub', body);
    msg.textContent = 'Generating code…';

    window.BlipNet.host(function (offerSdp) {
      window.BlipQR.render(canvas, offerSdp);
      msg.textContent = 'Have your friend tap JOIN, then point their camera at this code.';
      showScanButton(body, panel, 'SCAN THEIR ANSWER', function (text) {
        window.BlipNet.submitAnswer(text);
      });
    }, function (s) {
      if (s === 'connected') { setTimeout(dismissModal, 600); return; }
      if (s === 'failed' || s === 'timeout') {
        setTimeout(function () { if (modalEl) showChoice(body, panel); }, 1500);
      }
    });
  }

  function showJoinQR(body, panel) {
    clear(body);
    el('div', 'blip-hs-sub', body).textContent = 'Point your camera at the host’s code.';
    showScanButton(body, panel, 'SCAN HOST’S CODE', function (offerSdp) {
      clear(body);
      var canvas = qrCanvas(body);
      var msg = el('div', 'blip-hs-sub', body);
      msg.textContent = 'Generating your answer…';
      window.BlipNet.join(offerSdp, function (answerSdp) {
        window.BlipQR.render(canvas, answerSdp);
        msg.textContent = 'Show this to your friend — have them tap SCAN THEIR ANSWER.';
      }, function (s) {
        msg.textContent = statusText(s);
        if (s === 'connected') { setTimeout(dismissModal, 600); return; }
        if (s === 'failed' || s === 'timeout') {
          setTimeout(function () { if (modalEl) showChoice(body, panel); }, 1500);
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
      var wrap = el('div', '', holder);
      wrap.style.cssText = 'position:relative;width:220px;height:220px;margin:10px auto;';
      var video = el('video', '', wrap);
      video.setAttribute('playsinline', '');
      video.setAttribute('muted', '');
      video.muted = true;
      video.style.cssText = 'display:block;position:absolute;inset:0;width:220px;height:220px;object-fit:cover;border-radius:4px;background:#000;';
      // Drawn on top of the video by blip_qr.js's scan() — a green box on
      // a confirmed decode, yellow circles on whatever its finder-pattern
      // heuristic currently thinks might be a code ("mark it if you see
      // it"). Classed apart from .blip-qr-canvas (the rendered-code
      // canvas) so nothing querying for one can pick up the other.
      var overlay = el('canvas', 'blip-scan-overlay', wrap);
      overlay.width = 220;
      overlay.height = 220;
      overlay.style.cssText = 'display:block;position:absolute;inset:0;width:220px;height:220px;pointer-events:none;';
      var status = el('div', 'blip-hs-sub blip-scan-status', holder);
      status.textContent = 'Opening camera…';
      var err = el('div', 'blip-hs-err', holder);
      stopActiveScan = window.BlipQR.scan(video, overlay, function (text, scanErr) {
        stopActiveScan = null;
        if (scanErr) { err.textContent = scanErrorMessage(scanErr); return; }
        clear(holder);
        onScanned(text);
      }, function (state, detail) {
        status.textContent = scanStatusText(state, detail);
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
