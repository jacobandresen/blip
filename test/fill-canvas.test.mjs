import { test } from 'node:test';
import assert from 'node:assert/strict';

// Logic extracted from shell.html fillCanvas().
// Canvas has position:fixed top:50% left:50% with transform-origin:0 0.
// The transform places the canvas top-left at (vw/2, vh/2) then shifts it.
function computeTransform(canvasW, canvasH, viewportW, viewportH) {
  const scale = Math.min(viewportW / canvasW, viewportH / canvasH);
  const tx = -(canvasW * scale / 2);
  const ty = -(canvasH * scale / 2);
  return { scale, tx, ty };
}

function canvasRect(canvasW, canvasH, viewportW, viewportH, tx, ty, scale) {
  const left   = viewportW / 2 + tx;
  const top    = viewportH / 2 + ty;
  return { left, top, right: left + canvasW * scale, bottom: top + canvasH * scale };
}

function assertCentered(rect, width, height) {
  assert.ok(Math.abs((rect.left + rect.right) / 2 - width / 2) < 0.001);
  assert.ok(Math.abs((rect.top + rect.bottom) / 2 - height / 2) < 0.001);
}

const CASES = [
  { cw: 640, ch: 480, vw: 1280, vh: 960  },  // 2× upscale
  { cw: 640, ch: 480, vw:  800, vh: 600  },  // slight scale > 1
  { cw: 640, ch: 480, vw:  320, vh: 240  },  // 0.5× downscale
  { cw: 640, ch: 480, vw: 1920, vh: 1080 },  // widescreen, letterboxed
  { cw: 320, ch: 240, vw: 1920, vh: 1080 },  // small canvas, large viewport
];

test('canvas center equals viewport center', () => {
  for (const { cw, ch, vw, vh } of CASES) {
    const { scale, tx, ty } = computeTransform(cw, ch, vw, vh);
    const r = canvasRect(cw, ch, vw, vh, tx, ty, scale);
    assertCentered(r, vw, vh);
  }
});

test('canvas stays within viewport bounds', () => {
  for (const { cw, ch, vw, vh } of CASES) {
    const { scale, tx, ty } = computeTransform(cw, ch, vw, vh);
    const r = canvasRect(cw, ch, vw, vh, tx, ty, scale);
    assert.ok(r.left   >= -0.001, `left off-screen: ${r.left}`);
    assert.ok(r.top    >= -0.001, `top off-screen: ${r.top}`);
    assert.ok(r.right  <= vw + 0.001, `right off-screen: ${r.right} > ${vw}`);
    assert.ok(r.bottom <= vh + 0.001, `bottom off-screen: ${r.bottom} > ${vh}`);
  }
});

test('canvas dimensions must be known before computing its transform', () => {
  const vw = 375, vh = 667;
  const defaultSize = computeTransform(300, 150, vw, vh);
  const gameSize = computeTransform(480, 540, vw, vh);
  const wrongRect = canvasRect(480, 540, vw, vh,
    defaultSize.tx, defaultSize.ty, defaultSize.scale);
  assert.ok(
    wrongRect.right > vw || wrongRect.bottom > vh,
    'a transform computed for different canvas dimensions should overflow'
  );
  assertCentered(canvasRect(480, 540, vw, vh,
    gameSize.tx, gameSize.ty, gameSize.scale), vw, vh);
});
