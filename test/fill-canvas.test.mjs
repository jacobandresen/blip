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
    const cx = (r.left + r.right)  / 2;
    const cy = (r.top  + r.bottom) / 2;
    assert.ok(Math.abs(cx - vw / 2) < 0.001,
      `x center wrong for ${cw}×${ch} in ${vw}×${vh}: ${cx} ≠ ${vw/2}`);
    assert.ok(Math.abs(cy - vh / 2) < 0.001,
      `y center wrong for ${cw}×${ch} in ${vw}×${vh}: ${cy} ≠ ${vh/2}`);
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

// Regression: if fillCanvas is computed against HTML default dimensions (300×150)
// instead of the real game dimensions set by SDL (e.g. 480×540), the canvas
// ends up far off-screen. The fix is to use a MutationObserver so fillCanvas
// fires when SDL writes the real dimensions, not on the default 300×150.
test('transform computed on SDL dimensions differs from one on HTML defaults', () => {
  const vw = 375, vh = 667; // typical phone
  const htmlDefault = computeTransform(300, 150, vw, vh);  // wrong — fires before SDL
  const gameReal    = computeTransform(480, 540, vw, vh);  // correct — after SDL sets

  // The wrong transform (calibrated for 300×150) positions a 480×540 canvas badly
  const wrongRect = canvasRect(480, 540, vw, vh, htmlDefault.tx, htmlDefault.ty, htmlDefault.scale);
  assert.ok(
    wrongRect.right > vw || wrongRect.bottom > vh,
    'canvas calibrated for HTML defaults should overflow the viewport when real dims are used'
  );

  // The correct transform centers the real canvas
  const goodRect = canvasRect(480, 540, vw, vh, gameReal.tx, gameReal.ty, gameReal.scale);
  const cx = (goodRect.left + goodRect.right) / 2;
  const cy = (goodRect.top + goodRect.bottom) / 2;
  assert.ok(Math.abs(cx - vw / 2) < 0.001, `x off-center: ${cx}`);
  assert.ok(Math.abs(cy - vh / 2) < 0.001, `y off-center: ${cy}`);
});

// Regression: the reverted commit used -(w/2) instead of -(w*scale/2).
// When scale ≠ 1 this shifts the canvas off-center and partially off-screen.
test('buggy -(w/2) formula fails when scale != 1', () => {
  const { cw, ch, vw, vh } = { cw: 640, ch: 480, vw: 1280, vh: 960 };
  const scale   = Math.min(vw / cw, vh / ch); // 2.0
  const buggyTx = -(cw / 2);
  const buggyTy = -(ch / 2);
  const r = canvasRect(cw, ch, vw, vh, buggyTx, buggyTy, scale);
  const cx = (r.left + r.right) / 2;
  assert.notEqual(Math.round(cx), vw / 2,
    'buggy formula should NOT center the canvas');
});
