// The arcade room behind the cabinets (web/arcade.css): a plain wall and floor, a shadow behind each machine,
// nothing in the cabinets' way, still at rest, and a walk that counts on round the loop without a jump.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openArcade } from './lib/harness.mjs';

const CONFIGS = [
  { name: 'phone portrait', width: 390, height: 844, touch: true },
  { name: 'touch landscape', width: 844, height: 390, touch: true },
  { name: 'desktop', width: 1280, height: 800, touch: false },
  { name: 'kiosk', width: 1080, height: 1920, touch: false, fullscreen: true },
];

const settled = (page) => page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'),
  null, { timeout: 15000 });
const roomTravel = (page) => page.evaluate(() => Number(document.getElementById('game-grid').dataset.roomTravel));

for (const config of CONFIGS) {
  test(`the room is a wall, a floor and the machines' shadows on ${config.name}`, async (t) => {
    const { page } = await openArcade(t, config);
    const room = await page.evaluate(() => {
      const grid = document.getElementById('game-grid'), r = grid.getBoundingClientRect();
      const floor = parseFloat(grid.style.getPropertyValue('--room-floor-y'));
      const extras = [...grid.children].filter((el) => !el.classList.contains('card') && getComputedStyle(el).display !== 'none')
        .map((el) => el.className || el.tagName);
      const shadows = [...grid.querySelectorAll('.card-backlight')].map((el) => ({
        hidden: el.getAttribute('aria-hidden'), pointer: getComputedStyle(el).pointerEvents }));
      return { floor, height: r.height, close: grid.hasAttribute('data-close'), extras, shadows, cards: grid.querySelectorAll('.card').length };
    });
    // Close up (a phone on its side) the machines are cut off below Start, and the floor with them.
    assert.ok(room.floor > 0 && (room.close || room.floor < room.height), `the floor meets the wall inside the room (${room.floor} of ${room.height})`);
    assert.deepEqual(room.extras, [], 'nothing but machines stands in the room');
    assert.equal(room.shadows.length, room.cards, 'each machine casts a shadow');
    for (const s of room.shadows) assert.deepEqual(s, { hidden: 'true', pointer: 'none' });
  });
}

for (const [width, height, touch] of [[390, 844, true], [1280, 800, false]]) {
  test(`a press on the screen, Start or coin door still reaches the cabinet at ${width}×${height}`, async (t) => {
    const { page } = await openArcade(t, { width, height, touch });
    const hits = await page.evaluate(() => {
      const card = document.querySelector('.card-focused');
      return ['.game-rom', '.machine-start', '.machine-coinbox'].map((sel) => {
        const r = card.querySelector(sel).getBoundingClientRect();
        const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return [sel, !!(hit && hit.closest('.card-focused')), hit && (hit.className || hit.tagName)];
      });
    });
    for (const [sel, inside, what] of hits) assert.ok(inside, `${sel}'s centre hits the cabinet, not ${what}`);
  });
}

test('the room keeps still at rest', async (t) => {
  const { page } = await openArcade(t);
  await page.waitForTimeout(1500);
  const running = await page.evaluate(() => document.getAnimations().filter((a) => {
    const target = a.effect && a.effect.target;
    return a.playState === 'running' && target && (target.id === 'game-grid' || target.classList.contains('card-backlight'));
  }).length);
  assert.equal(running, 0);
});

test('the walk counts on past the last machine and the first, both ways, without a jump', async (t) => {
  const { page } = await openArcade(t);
  await page.evaluate(() => {
    window.__travel = [];
    const grid = document.getElementById('game-grid');
    (function tick() {
      window.__travel.push({ travel: Number(grid.dataset.roomTravel), pos: Number(grid.dataset.aislePos) });
      if (window.__travel.length < 4000) requestAnimationFrame(tick);
    })();
  });
  await page.keyboard.press('ArrowLeft');
  await settled(page);
  assert.ok(Math.abs((await roomTravel(page)) + 1) < 0.01, `walked one machine back: ${await roomTravel(page)}`);
  await page.keyboard.press('ArrowRight');
  await settled(page);
  assert.ok(Math.abs(await roomTravel(page)) < 0.01, `and back to the start: ${await roomTravel(page)}`);
  for (const [key, mark, dir] of [['ArrowRight', 9.4, 1], ['ArrowLeft', 0.4, -1]]) {
    await page.keyboard.down(key);
    await page.waitForFunction(([mark, dir]) => {
      const travel = Number(document.getElementById('game-grid').dataset.roomTravel);
      return dir > 0 ? travel > mark : travel < mark;
    }, [mark, dir], { timeout: 20000 });
    await page.keyboard.up(key);
    await settled(page);
  }
  const samples = await page.evaluate(() => window.__travel.slice());
  assert.ok(samples.some((s, i) => i && Math.abs(s.pos - samples[i - 1].pos) > 4), 'the aisle wrapped round while sampled');
  for (let i = 1; i < samples.length; i++) {
    assert.ok(Math.abs(samples[i].travel - samples[i - 1].travel) < 1, `travel ${samples[i - 1].travel} → ${samples[i].travel}`);
  }
});
