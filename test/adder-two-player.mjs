import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('Adder opens the second station and joins a human player from the title', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: true, viewport: { width: 390, height: 844 },
  });
  await page.addInitScript(() => sessionStorage.setItem('blip-coins', '4'));
  await page.goto(`${origin}/adder/index.html?controls=pad`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  assert.equal(await page.locator('html').getAttribute('data-players'), '2',
    'the second station must be available before the game starts');

  await page.locator('#glcanvas').focus();
  await page.keyboard.press('Space');
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '3');
  const p2Join = page.locator('#snes-pad-p2 .snes-btn').first();
  await p2Join.waitFor({ state: 'visible' });
  const joinBox = await p2Join.boundingBox();
  assert.ok(joinBox && joinBox.width > 0, 'the player-two join button must be visible');
  await page.touchscreen.tap(joinBox.x + joinBox.width / 2, joinBox.y + joinBox.height / 2);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'));
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '2');
  assert.equal(await page.locator('html').getAttribute('data-players'), '2');
});
