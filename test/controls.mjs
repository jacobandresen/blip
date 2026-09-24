// The touch controls, where precision is actually felt.
//
// Rally's paddle is a dial: the player rotates a knob with one finger and
// the rotation is turned into the same key events the keyboard produces.
// How that conversion treats a *slow* rotation decides whether careful
// aiming is possible, and it used to make it impossible.
//
// The dial thresholded each individual pointermove against a dead zone.
// Any rotation slower than one dead zone per event -- which is exactly the
// deliberate, careful turn a player makes when lining up a shot -- was
// discarded entirely, while a fast flick worked fine. Worse, a single
// small delta part-way through a turn *released* the key, so even a steady
// turn stuttered. Rotation is accumulated now.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, loadRally, evaluate } from './lib/harness.mjs';

const ENGINE = process.env.BLIP_CONTROLS_ENGINE || 'chromium';

/** Rotate a bound dial by `steps` increments of `perStep` radians, and
 * report which keys the controller emitted onto the game canvas. */
const ROTATE = (perStep, steps) => `
  new Promise(function (resolve) {
    var dial = document.createElement('div');
    dial.style.cssText = 'position:fixed;left:100px;top:100px;width:120px;height:120px;';
    document.body.appendChild(dial);

    var seen = [];
    var canvas = document.getElementById('glcanvas');
    canvas.addEventListener('keydown', function (e) { seen.push('down:' + e.code); });
    canvas.addEventListener('keyup', function (e) { seen.push('up:' + e.code); });

    window.BlipController.bindDial(dial, {
      up:   { key: 'i', code: 'KeyI' },
      down: { key: 'k', code: 'KeyK' }
    });

    var cx = 160, cy = 160, r = 50;
    function at(angle) {
      return { x: cx + Math.cos(angle) * r, y: cy + Math.sin(angle) * r };
    }
    function send(type, angle, id) {
      var p = at(angle);
      dial.dispatchEvent(new PointerEvent(type, {
        bubbles: true, cancelable: true, pointerId: id, clientX: p.x, clientY: p.y
      }));
    }

    var angle = 0;
    send('pointerdown', angle, 1);
    for (var i = 0; i < ${steps}; i++) {
      angle += ${perStep};
      send('pointermove', angle, 1);
    }
    // Let the idle-release timer settle before reporting.
    setTimeout(function () {
      send('pointerup', angle, 1);
      resolve({ seen: seen, downs: seen.filter(function (s) { return s.indexOf('down:') === 0; }).length });
    }, 200);
  })`;

test(`the paddle dial registers a slow, careful rotation (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);
  await loadRally(cdp);

  await t.test('a rotation slower than the dead zone per event still moves the paddle', async () => {
    // 0.006 rad per event against a 0.018 dead zone: every single event is
    // below threshold, so the old per-event test emitted nothing at all no
    // matter how far the finger actually travelled. Twelve of them is
    // 0.072 rad -- four dead zones of real rotation.
    const r = await evaluate(cdp, ROTATE(0.006, 12));
    t.diagnostic(`slow rotation emitted: ${JSON.stringify(r.seen)}`);
    assert.ok(r.downs > 0,
      'a slow rotation emitted no key at all — fine adjustment is impossible');
  });

  await t.test('it moves in the direction the finger actually turned', async () => {
    const forward = await evaluate(cdp, ROTATE(0.006, 12));
    const backward = await evaluate(cdp, ROTATE(-0.006, 12));
    const dirOf = (r) => (r.seen.find((s) => s.indexOf('down:') === 0) || '').replace('down:', '');
    t.diagnostic(`forward -> ${dirOf(forward)}, backward -> ${dirOf(backward)}`);
    assert.ok(dirOf(forward) && dirOf(backward), 'one of the directions emitted nothing');
    assert.notEqual(dirOf(forward), dirOf(backward),
      'turning the dial both ways produced the same key');
  });

  await t.test('a fast rotation still works, unchanged', async () => {
    // The case that always worked: one event well past the dead zone.
    const r = await evaluate(cdp, ROTATE(0.05, 6));
    assert.ok(r.downs > 0, 'a fast rotation stopped working');
  });

  await t.test('the key is released once the finger stops', async () => {
    // Holding a direction forever after the finger stopped would be worse
    // than the bug being fixed: the paddle would run off on its own.
    const r = await evaluate(cdp, ROTATE(0.006, 12));
    const lastDown = r.seen.lastIndexOf(r.seen.filter((s) => s.startsWith('down:')).pop());
    const ups = r.seen.map((s, i) => (s.startsWith('up:') ? i : -1)).filter((i) => i > lastDown);
    assert.ok(ups.length > 0, `no key release after the rotation stopped: ${JSON.stringify(r.seen)}`);
  });
});
