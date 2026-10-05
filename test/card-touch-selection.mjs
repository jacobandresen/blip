import test from 'node:test';
import assert from 'node:assert/strict';
import { startPage } from './lib/harness.mjs';

test('touch swipes move the focused game card in either direction', async (t) => {
  const handle = await startPage('chromium', {
    hasTouch: true, viewport: { width: 390, height: 844 },
  });
  t.after(handle.close);
  const { page, origin } = handle;
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-serpent')?.offsetWidth > 0);
  await page.evaluate(() => saveCoins(1));

  const rack = page.locator('#game-grid');
  const box = await rack.boundingBox();
  const y = box.y + box.height / 2;
  await page.evaluate(() => {
    window.__cardPointerLog = [];
    for (const type of ['pointerdown', 'pointerup', 'pointercancel']) {
      document.querySelector('#game-grid').addEventListener(type, (event) =>
        window.__cardPointerLog.push({ type, pointerType: event.pointerType, x: event.clientX, y: event.clientY }));
    }
  });
  const swipe = async (fromX, toX) => {
    const touch = await page.context().newCDPSession(page);
    try {
      await touch.send('Input.dispatchTouchEvent', {
        type: 'touchStart', touchPoints: [{ id: 1, x: fromX, y }],
      });
      await touch.send('Input.dispatchTouchEvent', {
        type: 'touchMove', touchPoints: [{ id: 1, x: toX, y }],
      });
      await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    } finally {
      await touch.detach();
    }
  };
  const focused = () => page.locator('#game-grid .card-focused .card-title').textContent();
  const initial = await focused();

  await swipe(box.x + box.width * 0.78, box.x + box.width * 0.22);
  try {
    await page.waitForFunction((before) =>
      document.querySelector('#game-grid .card-focused .card-title')?.textContent !== before, initial, { timeout: 3000 });
  } catch {
    assert.fail(`left swipe did not move focus; pointer events: ${JSON.stringify(await page.evaluate(() => window.__cardPointerLog))}`);
  }
  await page.waitForTimeout(450);
  const afterLeft = await focused();
  assert.notEqual(afterLeft, initial, 'a leftward swipe changes the selected card');

  await swipe(box.x + box.width * 0.22, box.x + box.width * 0.78);
  await page.waitForFunction((before) =>
    document.querySelector('#game-grid .card-focused .card-title')?.textContent !== before, afterLeft);
  await page.waitForTimeout(450);
  assert.equal(await focused(), initial, 'a rightward swipe returns to the previous card');

  await swipe(box.x + box.width * 0.84, box.x + box.width * 0.16);
  await page.waitForTimeout(900);
  const afterLongSwipe = await focused();
  assert.notEqual(afterLongSwipe, initial, 'a longer swipe can step across multiple cards');
});
