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
   * in. */
  function render(canvas, text) {
    var qr = qrcode(0, 'L');
    qr.addData(text);
    qr.make();
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
  // works or times out. So this reimplements the coarse first step of
  // jsQR's own locator (web/vendor/jsQR.js's `locate()`): scan rows for the
  // classic 1:1:3:1:1 dark:light:dark:light:dark run-length ratio that
  // marks a QR finder pattern, using a single global brightness threshold
  // instead of jsQR's own per-region adaptive one, and without jsQR's
  // column cross-check or quad-matching across rows. That makes it cheaper
  // and noticeably less precise than the real decoder — it WILL flag
  // things that aren't actually QR codes sometimes — but it only ever
  // drives a "maybe here" marker, never the actual pairing decision, so
  // false positives cost nothing but a stray yellow circle for a frame or
  // two.
  var FINDER_ROW_STRIDE = 4; // scan every 4th row — plenty dense at normal scanning distance, far cheaper than every row

  function luma(data, i) {
    return 0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2];
  }

  function findFinderCandidates(frame) {
    var data = frame.data, w = frame.width, h = frame.height;
    var lo = 255, hi = 0;
    for (var sy = 0; sy < h; sy += FINDER_ROW_STRIDE * 3) {
      for (var sx = 0; sx < w; sx += 5) {
        var l = luma(data, (sy * w + sx) * 4);
        if (l < lo) lo = l;
        if (l > hi) hi = l;
      }
    }
    if (hi - lo < 40) return []; // near-flat frame (lens cap, darkness, blank wall) — nothing to find, don't even try
    var threshold = (lo + hi) / 2;
    var hits = [];
    for (var y = 0; y < h; y += FINDER_ROW_STRIDE) {
      var runs = [0, 0, 0, 0, 0];
      var runLen = 0;
      var lastDark = false;
      for (var x = 0; x <= w; x++) {
        var dark = x < w && luma(data, (y * w + x) * 4) < threshold;
        if (x === 0) { lastDark = dark; runLen = 1; continue; }
        if (dark === lastDark) { runLen++; continue; }
        runs = [runs[1], runs[2], runs[3], runs[4], runLen];
        runLen = 1;
        lastDark = dark;
        var avg = (runs[0] + runs[1] + runs[2] + runs[3] + runs[4]) / 7; // 1+1+3+1+1 = 7 units total
        if (avg >= 1 &&
            Math.abs(runs[0] - avg) < avg && Math.abs(runs[1] - avg) < avg &&
            Math.abs(runs[2] - 3 * avg) < 3 * avg &&
            Math.abs(runs[3] - avg) < avg && Math.abs(runs[4] - avg) < avg &&
            !dark // a finder pattern is bordered in light, so we should be back on a light run now
        ) {
          hits.push({ x: x - runs[3] - runs[4] - runs[2] / 2, y: y });
        }
      }
    }
    // Several adjacent scan rows will each fire on the same real finder
    // square — cluster nearby hits into one marker rather than a smear of
    // them.
    var clusters = [];
    hits.forEach(function (p) {
      var c = null;
      for (var i = 0; i < clusters.length; i++) {
        if (Math.abs(clusters[i].x - p.x) < 24 && Math.abs(clusters[i].y - p.y) < 24) { c = clusters[i]; break; }
      }
      if (c) { c.x = (c.x * c.n + p.x) / (c.n + 1); c.y = (c.y * c.n + p.y) / (c.n + 1); c.n++; }
      else clusters.push({ x: p.x, y: p.y, n: 1 });
    });
    return clusters.filter(function (c) { return c.n >= 3; });
  }

  // ---- marker overlay --------------------------------------------------------
  //
  // Points from jsQR's `location` and from findFinderCandidates() above are
  // both in the *captured frame's* native pixel space (the video's own
  // width/height). The <video> itself is shown with `object-fit: cover`,
  // which crops-and-scales rather than stretching, so mapping a frame point
  // onto the overlay canvas — sized to the video's on-screen CSS box — has
  // to replicate that same crop math or every marker would land in the
  // wrong spot.
  function coverMap(px, py, nativeW, nativeH, dispW, dispH) {
    var scale = Math.max(dispW / nativeW, dispH / nativeH);
    var offX = (dispW - nativeW * scale) / 2;
    var offY = (dispH - nativeH * scale) / 2;
    return { x: px * scale + offX, y: py * scale + offY };
  }

  function drawMarkers(overlay, nativeW, nativeH, found, candidates) {
    var octx = overlay.getContext('2d');
    var dispW = overlay.width, dispH = overlay.height;
    octx.clearRect(0, 0, dispW, dispH);
    if (found) {
      var corners = [found.topLeftCorner, found.topRightCorner, found.bottomRightCorner, found.bottomLeftCorner];
      octx.strokeStyle = '#3ecf5a';
      octx.lineWidth = 3;
      octx.beginPath();
      corners.forEach(function (c, i) {
        var m = coverMap(c.x, c.y, nativeW, nativeH, dispW, dispH);
        if (i === 0) octx.moveTo(m.x, m.y); else octx.lineTo(m.x, m.y);
      });
      octx.closePath();
      octx.stroke();
      return;
    }
    if (!candidates.length) return;
    octx.strokeStyle = '#e8c547';
    octx.lineWidth = 2;
    octx.setLineDash([4, 3]);
    candidates.forEach(function (c) {
      var m = coverMap(c.x, c.y, nativeW, nativeH, dispW, dispH);
      octx.beginPath();
      octx.arc(m.x, m.y, 16, 0, Math.PI * 2);
      octx.stroke();
    });
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
   * decode, or yellow circles on whatever findFinderCandidates() above
   * currently thinks might be a QR pattern — "mark it if you see it".
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
        scratch.width = nativeW;
        scratch.height = nativeH;
        sctx.drawImage(videoEl, 0, 0, nativeW, nativeH);
        var frame = sctx.getImageData(0, 0, nativeW, nativeH);
        frames++;
        var code = null, decodeErr = null;
        try { code = jsQR(frame.data, frame.width, frame.height); } catch (e) { decodeErr = e; /* a torn frame — just try the next one */ }
        if (code && code.data) {
          if (overlayCanvas) drawMarkers(overlayCanvas, nativeW, nativeH, code.location, []);
          report('found', { frames: frames });
          var found = code.data;
          // Let the confirmation box actually show for one beat before the
          // camera tears down out from under it.
          setTimeout(function () { stop(); onScanned(found, null); }, 150);
          return;
        }
        var candidates = findFinderCandidates(frame);
        if (overlayCanvas) drawMarkers(overlayCanvas, nativeW, nativeH, null, candidates);
        report('scanning', {
          frames: frames,
          width: nativeW,
          height: nativeH,
          candidates: candidates.length,
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
        stopped = true;
        report('camera-error', { name: err && err.name, message: err && err.message });
        onScanned(null, err);
      });

    return stop;
  }

  window.BlipQR = { render: render, scan: scan };
}());
