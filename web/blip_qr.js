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

  // The QR spec requires a "quiet zone" — a plain light margin at least 4
  // modules wide around the code — for a scanner to reliably even find
  // the finder patterns in the first place, let alone decode. Skipping it
  // was one real cause of "nothing happens when I point the camera at
  // it": drawing the modules edge-to-edge left the code sitting directly
  // against the pairing modal's own dark background with no light border
  // at all, which jsQR (like most real QR scanners) can fail to detect
  // against entirely rather than just decode less reliably.
  var QUIET_ZONE_MODULES = 4;

  /** Draw `text` as a QR code onto `canvas`, quiet zone included. Low
   * error-correction — the two screens involved are only ever a few
   * inches apart under the scanning phone's own camera, not a printed
   * code exposed to real wear, so capacity (fitting a full SDP, ICE
   * candidates included) matters more here than damage tolerance. Auto
   * type-number (0) picks the smallest QR version the text actually fits
   * in — the one way this can still fail is `text` overflowing even the
   * largest QR version (2,953 bytes at this error-correction level), in
   * which case the vendored encoder throws a bare *string* (not an
   * `Error`); wrapped into a real `Error` here so every caller gets one
   * consistent, catchable failure shape instead of two different ones. */
  function render(canvas, text) {
    var qr = qrcode(0, 'L');
    qr.addData(text);
    try {
      qr.make();
    } catch (e) {
      throw new Error('QR encode failed: ' + e);
    }
    var count = qr.getModuleCount();
    var totalModules = count + QUIET_ZONE_MODULES * 2;
    var cell = Math.max(2, Math.floor(400 / totalModules));
    var size = cell * totalModules;
    var offset = QUIET_ZONE_MODULES * cell;
    canvas.width = size;
    canvas.height = size;
    var ctx = canvas.getContext('2d');
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, size, size); // also paints the quiet zone itself
    ctx.fillStyle = '#000';
    for (var r = 0; r < count; r++) {
      for (var c = 0; c < count; c++) {
        if (qr.isDark(r, c)) ctx.fillRect(offset + c * cell, offset + r * cell, cell, cell);
      }
    }
  }

  // ---- "do I see something QR-shaped yet?" heuristic ------------------------
  //
  // jsQR only ever reports a full, successful decode or `null` — there's no
  // public hook for "I can see a QR-like pattern but haven't read it yet",
  // which is exactly the feedback asked for ("can you mark the QR if you
  // see it?") so a real scanning attempt isn't just silence until it either
  // works or times out.
  //
  // An earlier version of this hunted for small-scale finder-pattern
  // features (the 1:1:3:1:1 dark:light:dark:light:dark run-length ratio
  // jsQR's own locate() looks for) scattered anywhere in the frame. Out in
  // the real world that flagged *hundreds* of false positives a frame —
  // ordinary background detail, on-screen text, and camera sensor noise
  // all produce small-scale matches — expensive enough (hundreds of
  // overlay shapes to draw, hundreds of entries for an O(n²) clustering
  // step, every single animation frame) to be a real suspect for scanning
  // failing outright rather than just showing a noisy overlay.
  //
  // This leans on the same assumption pairing already does: two phones
  // held close, the code framed to fill most of the screen it's shown on.
  // Rather than hunting for a small feature that could be anywhere, check
  // whether the one big *centered* region a code that size would actually
  // occupy — comfortably at least half the frame — has real QR-like
  // contrast in it. One region to check, not a fistful of small
  // candidates to chase, and its size is a feature, not a compromise: a
  // small high-contrast false positive (an icon, a line of text) never
  // reaches the size a candidate is required to be here.
  //
  // The actual pixel math (findFinderCandidate) and the marker-overlay
  // coordinate math (mapToDisplay) both live in web/blip_qr_heuristic.js —
  // pure functions with no DOM/canvas dependency, so they have real unit
  // tests (test/qr-heuristic.test.mjs) the same way blip_sdp_slim.js and
  // blip_net_proto.js do. Falls back to "never flags a candidate" (not a
  // hard crash) if that script failed to load — this feature is cosmetic;
  // losing it shouldn't take pairing down with it.
  var FINDER_SCAN_DIM = 200; // downsample target, applied to the already-square-cropped frame (see scan())
  var findFinderCandidate = (window.BlipQrHeuristic && window.BlipQrHeuristic.findFinderCandidate) || function () { return null; };
  var mapToDisplay = (window.BlipQrHeuristic && window.BlipQrHeuristic.mapToDisplay) || function (px, py) { return { x: px, y: py }; };

  // ---- marker overlay --------------------------------------------------------

  function drawMarkers(overlay, side, found, candidate) {
    var octx = overlay.getContext('2d');
    var dispSide = overlay.width; // overlay is always square — see blip_net_ui.js
    octx.clearRect(0, 0, overlay.width, overlay.height);
    if (found) {
      var corners = [found.topLeftCorner, found.topRightCorner, found.bottomRightCorner, found.bottomLeftCorner];
      octx.strokeStyle = '#3ecf5a';
      octx.lineWidth = 3;
      octx.beginPath();
      corners.forEach(function (c, i) {
        var m = mapToDisplay(c.x, c.y, side, dispSide);
        if (i === 0) octx.moveTo(m.x, m.y); else octx.lineTo(m.x, m.y);
      });
      octx.closePath();
      octx.stroke();
      return;
    }
    if (!candidate) return;
    var topLeft = mapToDisplay(candidate.x - candidate.side / 2, candidate.y - candidate.side / 2, side, dispSide);
    var boxSize = (candidate.side / side) * dispSide;
    octx.strokeStyle = '#e8c547';
    octx.lineWidth = 2;
    octx.setLineDash([6, 4]);
    octx.strokeRect(topLeft.x, topLeft.y, boxSize, boxSize);
    octx.setLineDash([]);
  }

  /**
   * Open the camera into `videoEl` and scan every frame until one QR code
   * decodes, then call `onScanned(text)` and stop the camera on its own.
   * `onScanned(null, error)` instead if the camera itself can't be opened
   * (permission denied, no camera, insecure context) — `error.name` is a
   * real DOMException name (`NotAllowedError`, `NotFoundError`, …) the
   * caller can turn into a specific message.
   *
   * `overlayCanvas` (optional) gets a live green box drawn on a confirmed
   * decode, or a dashed yellow square over whatever findFinderCandidate()
   * above currently thinks might be a QR pattern — "mark it if you see it".
   * `onStatus(state, detail)` (optional) fires continuously as scanning
   * progresses ('opening', 'streaming', 'buffering', 'scanning', 'found',
   * 'play-error', 'unsupported') so the caller can show live text instead
   * of silence while nothing has decoded yet.
   *
   * Returns a `stop()` the caller can invoke to cancel early (navigating
   * away from the scan screen, closing the pairing modal, …) — always safe
   * to call, including after a scan has already completed.
   */
  function scan(videoEl, overlayCanvas, onScanned, onStatus) {
    var stopped = false;
    var stream = null;
    var raf = null;
    var frames = 0;
    var startedAt = Date.now();
    var scratch = document.createElement('canvas');
    var sctx = scratch.getContext('2d', { willReadFrequently: true });
    // The finder-candidate heuristic's own downsampled working copy —
    // deliberately a *second*, small canvas rather than reusing `scratch`
    // at its (cropped, but still often much larger) size — see
    // findFinderCandidate()'s comment on why.
    var finderCanvas = document.createElement('canvas');
    var fctx = finderCanvas.getContext('2d', { willReadFrequently: true });

    function report(state, detail) {
      if (typeof onStatus === 'function') {
        try { onStatus(state, detail || {}); } catch (e) { /* a listener's own bug shouldn't break scanning */ }
      }
    }

    function stop() {
      if (stopped) return;
      stopped = true;
      if (raf) cancelAnimationFrame(raf);
      if (stream) stream.getTracks().forEach(function (t) { t.stop(); });
      if (videoEl) videoEl.srcObject = null;
      if (overlayCanvas) overlayCanvas.getContext('2d').clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);
    }

    function tick() {
      if (stopped) return;
      if (videoEl.readyState >= videoEl.HAVE_CURRENT_DATA && videoEl.videoWidth) {
        var nativeW = videoEl.videoWidth, nativeH = videoEl.videoHeight;
        // Crop to the largest centered *square* — the same crop
        // `object-fit: cover` already applies to the <video> element
        // itself when filling its own square CSS box (see
        // web/blip_net_ui.js's scan-preview markup), and the same "the
        // code fills the screen" assumption findFinderCandidate() leans
        // on. Scanning the full, wider/taller raw camera frame would
        // waste effort on — and risk false-flagging — content the user
        // can't even see in their own preview. A QR code is square, too,
        // so a square capture is the natural fit either way.
        var side = Math.min(nativeW, nativeH);
        var offsetX = Math.floor((nativeW - side) / 2);
        var offsetY = Math.floor((nativeH - side) / 2);
        scratch.width = side;
        scratch.height = side;
        sctx.drawImage(videoEl, offsetX, offsetY, side, side, 0, 0, side, side);
        var frame = sctx.getImageData(0, 0, side, side);
        frames++;
        var code = null, decodeErr = null;
        try { code = jsQR(frame.data, frame.width, frame.height); } catch (e) { decodeErr = e; /* a torn frame — just try the next one */ }
        if (code && code.data) {
          if (overlayCanvas) drawMarkers(overlayCanvas, side, code.location, null);
          report('found', { frames: frames });
          var found = code.data;
          // Let the confirmation box actually show for one beat before the
          // camera tears down out from under it.
          setTimeout(function () {
            // The caller's own stop() (returned below) can fire during this
            // beat — closing the pairing modal right as a decode lands, say
            // — in which case this scan has already been abandoned and
            // firing onScanned for it now would hand the caller a result it
            // never asked for any more.
            if (stopped) return;
            stop();
            onScanned(found, null);
          }, 150);
          return;
        }
        // Downsample the (already-cropped) frame for the heuristic — see
        // findFinderCandidate()'s own comment for why a small fixed size
        // beats scanning at native resolution.
        var fscale = Math.min(1, FINDER_SCAN_DIM / side);
        var smallSide = Math.max(1, Math.round(side * fscale));
        finderCanvas.width = smallSide;
        finderCanvas.height = smallSide;
        fctx.drawImage(videoEl, offsetX, offsetY, side, side, 0, 0, smallSide, smallSide);
        var smallFrame = fctx.getImageData(0, 0, smallSide, smallSide);
        var candidate = findFinderCandidate(smallFrame);
        if (candidate) {
          candidate = { x: candidate.x / fscale, y: candidate.y / fscale, side: candidate.side / fscale };
        }
        if (overlayCanvas) drawMarkers(overlayCanvas, side, null, candidate);
        report('scanning', {
          frames: frames,
          width: nativeW,
          height: nativeH,
          candidates: candidate ? 1 : 0,
          elapsedMs: Date.now() - startedAt,
          decodeError: decodeErr ? String((decodeErr && decodeErr.message) || decodeErr) : null,
        });
      } else {
        report('buffering', { readyState: videoEl.readyState });
      }
      raf = requestAnimationFrame(tick);
    }

    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
      report('unsupported');
      onScanned(null, new Error('no camera API'));
      return stop;
    }
    report('opening');
    // `ideal` (not `exact`) — a camera below this just gives its best,
    // never fails the request over it. Many browsers otherwise default to
    // a fairly low capture resolution (640x480-ish) unless asked for more,
    // which hands jsQR far fewer real pixels to find a small/dense code
    // in than the camera is actually capable of.
    navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'environment', width: { ideal: 1920 }, height: { ideal: 1920 } },
    })
      .then(function (s) {
        if (stopped) { s.getTracks().forEach(function (t) { t.stop(); }); return; }
        stream = s;
        videoEl.srcObject = s;
        videoEl.play().catch(function (e) { report('play-error', { message: e && e.message }); });
        var track = s.getVideoTracks()[0];
        var settings = track ? track.getSettings() : {};
        report('streaming', { width: settings.width, height: settings.height, deviceLabel: track ? track.label : '' });
        raf = requestAnimationFrame(tick);
      })
      .catch(function (err) {
        // Same staleness concern as the decode-success beat above: if the
        // caller already called the returned stop() while getUserMedia was
        // still pending (closed the modal before the permission prompt was
        // even answered, say), this rejection is for an abandoned attempt.
        if (stopped) return;
        stopped = true;
        report('camera-error', { name: err && err.name, message: err && err.message });
        onScanned(null, err);
      });

    return stop;
  }

  window.BlipQR = { render: render, scan: scan };
}());
