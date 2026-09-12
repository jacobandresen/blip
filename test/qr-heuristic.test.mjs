// Unit tests for web/blip_qr_heuristic.js — the pure pixel math behind
// blip_qr.js's "do I see something QR-shaped yet?" marker heuristic
// (docs/multiplayer.md). No DOM/canvas needed: a "frame" here is just a
// plain {data, width, height} object, the same shape a real ImageData
// has.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { FINDER_BOX_FRACTION, luma, findFinderCandidate, mapToDisplay } = require('../web/blip_qr_heuristic.js');

/** A flat-color w x h RGBA frame. */
function flatFrame(w, h, gray) {
  const data = new Uint8ClampedArray(w * h * 4);
  for (let i = 0; i < data.length; i += 4) {
    data[i] = data[i + 1] = data[i + 2] = gray;
    data[i + 3] = 255;
  }
  return { data, width: w, height: h };
}

/** The exact (x,y) coordinates findFinderCandidate() samples inside its
 * centered box, for a given w/h — mirrors its own box/step math so tests
 * can set precisely those pixels and know exactly how many samples will
 * land dark vs light, independent of anything outside the box. */
function sampledCoords(w, h) {
  const boxSide = Math.round(Math.min(w, h) * FINDER_BOX_FRACTION);
  const x0 = Math.floor((w - boxSide) / 2);
  const y0 = Math.floor((h - boxSide) / 2);
  const STEP = 4;
  const coords = [];
  for (let y = y0; y < y0 + boxSide; y += STEP) {
    for (let x = x0; x < x0 + boxSide; x += STEP) {
      coords.push([x, y]);
    }
  }
  return coords;
}

/** A white w x h frame with exactly `darkCount` of the sampled
 * coordinates (see sampledCoords) painted black — gives exact control
 * over the dark/light ratio findFinderCandidate() actually measures. */
function frameWithExactDarkSamples(w, h, darkCount) {
  const frame = flatFrame(w, h, 255);
  const coords = sampledCoords(w, h);
  assert.ok(darkCount <= coords.length, 'test setup error: more dark samples requested than exist');
  for (let i = 0; i < darkCount; i++) {
    const [x, y] = coords[i];
    const idx = (y * w + x) * 4;
    frame.data[idx] = frame.data[idx + 1] = frame.data[idx + 2] = 0;
  }
  return { frame, totalSamples: coords.length };
}

// ---- luma -------------------------------------------------------------

test('luma: pure black is 0, pure white is 255', () => {
  const black = new Uint8ClampedArray([0, 0, 0, 255]);
  const white = new Uint8ClampedArray([255, 255, 255, 255]);
  assert.equal(luma(black, 0), 0);
  assert.equal(Math.round(luma(white, 0)), 255);
});

test('luma: weights the green channel most heavily (perceptual, not a flat average)', () => {
  const pureGreen = new Uint8ClampedArray([0, 255, 0, 255]);
  const pureRed = new Uint8ClampedArray([255, 0, 0, 255]);
  assert.ok(luma(pureGreen, 0) > luma(pureRed, 0));
});

// ---- findFinderCandidate: rejections --------------------------------------

test('findFinderCandidate: a flat frame (no contrast at all) returns null', () => {
  const frame = flatFrame(80, 80, 128);
  assert.equal(findFinderCandidate(frame), null);
});

test('findFinderCandidate: pure black and pure white frames are both flat too', () => {
  assert.equal(findFinderCandidate(flatFrame(80, 80, 0)), null);
  assert.equal(findFinderCandidate(flatFrame(80, 80, 255)), null);
});

test('findFinderCandidate: contrast just under the flatness threshold (39 levels) returns null', () => {
  const frame = flatFrame(80, 80, 128);
  // Nudge exactly one sampled pixel to be 39 levels brighter — still
  // "near-flat" by the < 40 threshold.
  const [x, y] = sampledCoords(80, 80)[0];
  const idx = (y * 80 + x) * 4;
  frame.data[idx] = frame.data[idx + 1] = frame.data[idx + 2] = 128 + 39;
  assert.equal(findFinderCandidate(frame), null);
});

test('findFinderCandidate: almost entirely light (below the 0.25 dark-fraction floor) returns null', () => {
  const { frame, totalSamples } = frameWithExactDarkSamples(80, 80, 1);
  // 1 dark sample out of many is nowhere near a plausible QR mix, but
  // *is* enough contrast (0 vs 255) to pass the flatness check — this
  // isolates the dark-fraction rejection specifically.
  assert.ok(1 / totalSamples < 0.25);
  assert.equal(findFinderCandidate(frame), null);
});

test('findFinderCandidate: almost entirely dark (above the 0.75 dark-fraction ceiling) returns null', () => {
  const almostAllDark = sampledCoords(80, 80).length - 1;
  const { frame, totalSamples } = frameWithExactDarkSamples(80, 80, almostAllDark);
  assert.ok(almostAllDark / totalSamples > 0.75);
  assert.equal(findFinderCandidate(frame), null);
});

test('findFinderCandidate: just below the 0.25 boundary (12/49) returns null', () => {
  const { frame, totalSamples } = frameWithExactDarkSamples(40, 40, 12);
  assert.equal(totalSamples, 49); // pins the test's own assumption about the sample grid size
  assert.ok(12 / 49 < 0.25);
  assert.equal(findFinderCandidate(frame), null);
});

test('findFinderCandidate: just at/above the 0.25 boundary (13/49) is accepted', () => {
  const { frame, totalSamples } = frameWithExactDarkSamples(40, 40, 13);
  assert.equal(totalSamples, 49);
  assert.ok(13 / 49 > 0.25);
  assert.notEqual(findFinderCandidate(frame), null);
});

test('findFinderCandidate: just at/below the 0.75 boundary (36/49) is accepted', () => {
  const { frame, totalSamples } = frameWithExactDarkSamples(40, 40, 36);
  assert.equal(totalSamples, 49);
  assert.ok(36 / 49 < 0.75);
  assert.notEqual(findFinderCandidate(frame), null);
});

test('findFinderCandidate: just above the 0.75 boundary (37/49) returns null', () => {
  const { frame, totalSamples } = frameWithExactDarkSamples(40, 40, 37);
  assert.equal(totalSamples, 49);
  assert.ok(37 / 49 > 0.75);
  assert.equal(findFinderCandidate(frame), null);
});

// ---- findFinderCandidate: acceptance + shape ------------------------------

test('findFinderCandidate: a balanced half-dark/half-light box is accepted with the expected geometry', () => {
  const w = 100, h = 100;
  const frame = flatFrame(w, h, 255);
  // Left half of the frame dark, right half light — comfortably inside
  // the checked box either way, and close to a 50/50 mix.
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w / 2; x++) {
      const idx = (y * w + x) * 4;
      frame.data[idx] = frame.data[idx + 1] = frame.data[idx + 2] = 0;
    }
  }
  const candidate = findFinderCandidate(frame);
  assert.notEqual(candidate, null);
  const expectedSide = Math.round(Math.min(w, h) * FINDER_BOX_FRACTION);
  assert.equal(candidate.side, expectedSide);
  assert.equal(candidate.x, w / 2); // centered
  assert.equal(candidate.y, h / 2);
});

test('findFinderCandidate: is square even for a non-square frame (side is min(w,h)-based)', () => {
  const w = 200, h = 100;
  const frame = flatFrame(w, h, 255);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w / 2; x++) {
      const idx = (y * w + x) * 4;
      frame.data[idx] = frame.data[idx + 1] = frame.data[idx + 2] = 0;
    }
  }
  const candidate = findFinderCandidate(frame);
  assert.notEqual(candidate, null);
  assert.equal(candidate.side, Math.round(Math.min(w, h) * FINDER_BOX_FRACTION));
});

test('findFinderCandidate: works on a minimally small frame without throwing', () => {
  // Degenerate but real: scan()'s own downsample never goes below 1x1,
  // and a checkerboard-like single-sample box can't have "real contrast"
  // — this exercises that findFinderCandidate() itself never panics/
  // throws on the smallest possible input, whatever it returns.
  assert.doesNotThrow(() => findFinderCandidate(flatFrame(1, 1, 128)));
  assert.doesNotThrow(() => findFinderCandidate(flatFrame(4, 4, 128)));
});

// ---- mapToDisplay ----------------------------------------------------------

test('mapToDisplay: identity when side equals dispSide', () => {
  assert.deepEqual(mapToDisplay(10, 20, 100, 100), { x: 10, y: 20 });
});

test('mapToDisplay: scales up when the display box is bigger than the source frame', () => {
  assert.deepEqual(mapToDisplay(10, 10, 50, 100), { x: 20, y: 20 });
});

test('mapToDisplay: scales down when the display box is smaller than the source frame', () => {
  assert.deepEqual(mapToDisplay(20, 20, 100, 50), { x: 10, y: 10 });
});

test('mapToDisplay: the origin always maps to the origin', () => {
  assert.deepEqual(mapToDisplay(0, 0, 260, 248), { x: 0, y: 0 });
});
