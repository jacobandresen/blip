/* QR-code rendering + camera scanning for two-device pairing (the only
 * signaling path — see docs/multiplayer.md), wired up by
 * web/blip_net.js's host()/join() and web/blip_net_ui.js's modal.
 *
 * Two small vendored libraries do the actual work — this file is just the
 * glue between them and the DOM:
 *   web/vendor/qrcode.js — kazuhikoarase/qrcode-generator (MIT), draws an
 *     SDP string as a grid of dark/light modules.
 *   web/vendor/jsQR.js   — cozmo/jsQR (Apache-2.0), decodes a QR code out
 *     of raw pixel data.
 * Only defines window.BlipQR when both loaded — blip_net_ui.js checks for
 * that before showing PLAY NEARBY at all, so a missing/blocked vendor
 * script degrades to "no multiplayer button" rather than a broken one.
 */
(function () {
  'use strict';
  if (typeof qrcode !== 'function' || typeof jsQR !== 'function') return;

  /** Draw `text` as a QR code onto `canvas`. Low error-correction — the
   * two screens involved are only ever a few inches apart under the
   * scanning phone's own camera, not a printed code exposed to real wear,
   * so capacity (fitting a full SDP, ICE candidates included) matters
   * more here than damage tolerance. Auto type-number (0) picks the
   * smallest QR version the text actually fits in. */
  function render(canvas, text) {
    var qr = qrcode(0, 'L');
    qr.addData(text);
    qr.make();
    var count = qr.getModuleCount();
    var cell = Math.max(2, Math.floor(240 / count));
    var size = cell * count;
    canvas.width = size;
    canvas.height = size;
    var ctx = canvas.getContext('2d');
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, size, size);
    ctx.fillStyle = '#000';
    for (var r = 0; r < count; r++) {
      for (var c = 0; c < count; c++) {
        if (qr.isDark(r, c)) ctx.fillRect(c * cell, r * cell, cell, cell);
      }
    }
  }

  /**
   * Open the camera into `videoEl` and scan every frame until one QR code
   * decodes, then call `onScanned(text)` and stop the camera on its own.
   * `onScanned(null, error)` instead if the camera itself can't be
   * opened (permission denied, no camera, insecure context). Returns a
   * `stop()` the caller can invoke to cancel early (navigating away from
   * the scan screen, closing the pairing modal, …) — always safe to call,
   * including after a scan has already completed.
   */
  function scan(videoEl, onScanned) {
    var stopped = false;
    var stream = null;
    var raf = null;
    var scratch = document.createElement('canvas');
    var sctx = scratch.getContext('2d', { willReadFrequently: true });

    function stop() {
      if (stopped) return;
      stopped = true;
      if (raf) cancelAnimationFrame(raf);
      if (stream) stream.getTracks().forEach(function (t) { t.stop(); });
      if (videoEl) videoEl.srcObject = null;
    }

    function tick() {
      if (stopped) return;
      if (videoEl.readyState >= videoEl.HAVE_CURRENT_DATA && videoEl.videoWidth) {
        scratch.width = videoEl.videoWidth;
        scratch.height = videoEl.videoHeight;
        sctx.drawImage(videoEl, 0, 0, scratch.width, scratch.height);
        var frame = sctx.getImageData(0, 0, scratch.width, scratch.height);
        var code = null;
        try { code = jsQR(frame.data, frame.width, frame.height); } catch (e) { /* a torn frame — just try the next one */ }
        if (code && code.data) {
          var found = code.data;
          stop();
          onScanned(found, null);
          return;
        }
      }
      raf = requestAnimationFrame(tick);
    }

    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
      onScanned(null, new Error('no camera API'));
      return stop;
    }
    navigator.mediaDevices.getUserMedia({ video: { facingMode: 'environment' } })
      .then(function (s) {
        if (stopped) { s.getTracks().forEach(function (t) { t.stop(); }); return; }
        stream = s;
        videoEl.srcObject = s;
        videoEl.play().catch(function () {});
        raf = requestAnimationFrame(tick);
      })
      .catch(function (err) {
        stopped = true;
        onScanned(null, err);
      });

    return stop;
  }

  window.BlipQR = { render: render, scan: scan };
}());
