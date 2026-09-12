/* Pure pixel math for blip_qr.js's "do I see something QR-shaped yet?"
 * marker heuristic — no DOM, no canvas, no camera, just plain objects in
 * and out, so it can be unit-tested from Node (test/qr-heuristic.test.mjs)
 * the same way web/blip_sdp_slim.js and web/blip_net_proto.js are. Loaded
 * as a browser <script> by blip_qr.js the rest of the time, same dual-mode
 * export pattern as those two.
 *
 * See docs/multiplayer.md for the reasoning behind the approach itself
 * (why a single big centered square instead of hunting for small finder-
 * pattern features): in short, it leans on the same assumption pairing
 * already does — two phones held close, the code framed to fill most of
 * the screen — rather than searching the whole frame for a feature that
 * could be anywhere in it.
 */
(function (root) {
  'use strict';

  var FINDER_BOX_FRACTION = 0.7; // the checked region's side, as a fraction of the frame — comfortably over half

  function luma(data, i) {
    return 0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2];
  }

  /** `frame` is `{data, width, height}` — a real ImageData in the browser,
   * or (for tests) any plain object with a flat RGBA `data` array of
   * length `width*height*4`. Expected to already be cropped to a square
   * and downsampled by the caller (blip_qr.js's scan()). Returns one
   * candidate — the big centered box itself — when it looks QR-like, or
   * `null`. "Looks QR-like" just means real contrast, *and* a plausible
   * dark/light mix (a QR code is roughly half dark modules, half light —
   * a region that's almost entirely one or the other, like a plain wall
   * with a bright reflection on it, has contrast but isn't a plausible
   * code). */
  function findFinderCandidate(frame) {
    var data = frame.data, w = frame.width, h = frame.height;
    var boxSide = Math.round(Math.min(w, h) * FINDER_BOX_FRACTION);
    var x0 = Math.floor((w - boxSide) / 2);
    var y0 = Math.floor((h - boxSide) / 2);
    var STEP = 4;
    var lo = 255, hi = 0, dark = 0, total = 0;
    for (var y = y0; y < y0 + boxSide; y += STEP) {
      for (var x = x0; x < x0 + boxSide; x += STEP) {
        var l = luma(data, (y * w + x) * 4);
        if (l < lo) lo = l;
        if (l > hi) hi = l;
        total++;
      }
    }
    if (hi - lo < 40) return null; // near-flat (lens cap, darkness, a blank wall) — nothing to find
    var threshold = (lo + hi) / 2;
    for (var y2 = y0; y2 < y0 + boxSide; y2 += STEP) {
      for (var x2 = x0; x2 < x0 + boxSide; x2 += STEP) {
        if (luma(data, (y2 * w + x2) * 4) < threshold) dark++;
      }
    }
    var darkFraction = dark / total;
    if (darkFraction < 0.25 || darkFraction > 0.75) return null;
    return { x: x0 + boxSide / 2, y: y0 + boxSide / 2, side: boxSide };
  }

  /** Map a point in a `side`x`side` cropped-square frame onto a
   * `dispSide`x`dispSide` square display box — a uniform scale, no
   * per-axis offset, because the caller (blip_qr.js's scan()) already
   * crops the frame to exactly the square `object-fit: cover` shows on
   * screen (see its own comment). */
  function mapToDisplay(px, py, side, dispSide) {
    var scale = dispSide / side;
    return { x: px * scale, y: py * scale };
  }

  var api = {
    FINDER_BOX_FRACTION: FINDER_BOX_FRACTION,
    luma: luma,
    findFinderCandidate: findFinderCandidate,
    mapToDisplay: mapToDisplay,
  };

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = api;
  } else {
    root.BlipQrHeuristic = api;
  }
}(typeof window !== 'undefined' ? window : this));
