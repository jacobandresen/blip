// A second player joins: on Adder's title by touch, on Warlock's title or mid-game with J. Each costs a credit.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, gameReady, PHONE } from './lib/harness.mjs';

test('Adder opens the second station and joins a human player from the title', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: true, viewport: PHONE,
  });
  await page.addInitScript(() => sessionStorage.setItem('blip-coins', '4'));
  await page.goto(`${origin}/adder/index.html`);
  await gameReady(page);
  assert.equal(await page.locator('html').getAttribute('data-players'), '2',
    'the second station must be available before the game starts');

  await page.locator('#glcanvas').focus();
  await page.keyboard.press('Space');
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '3');
  assert.equal(await page.evaluate(() => blipControls()), 'touch');
  const p2Join = page.locator('.ts-join');
  await p2Join.waitFor({ state: 'visible' });
  const joinBox = await p2Join.boundingBox();
  assert.ok(joinBox && joinBox.width > 0, 'the player-two join button must be visible');
  await page.touchscreen.tap(joinBox.x + joinBox.width / 2, joinBox.y + joinBox.height / 2);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'));
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '2');
  assert.equal(await page.locator('html').getAttribute('data-players'), '2');
});

// J (station two's first button) on Warlock's title starts a two-player game;
// in play it joins a second wizard. Each costs a credit.
async function warlockAt(t) {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.addInitScript(() => sessionStorage.setItem('blip-coins', '4'));
  await page.goto(`${origin}/warlock/index.html`);
  await gameReady(page);
  await page.locator('#glcanvas').focus();
  // the game reads keys from its first frames on; let it settle on the title
  await page.waitForTimeout(1500);
  await page.keyboard.press('KeyX');
  await page.waitForTimeout(500);
  return page;
}

test('Warlock: J on the title starts a two-player game', async (t) => {
  const page = await warlockAt(t);
  await page.keyboard.press('KeyJ');
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'), undefined, { timeout: 5000 });
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '2');
});

test('Warlock: J in play joins a second wizard', async (t) => {
  const page = await warlockAt(t);
  await page.keyboard.press('Space');
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '3');
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-versus')), false);
  await page.waitForTimeout(500);
  await page.keyboard.press('KeyJ');
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'), undefined, { timeout: 5000 });
  await page.waitForFunction(() => sessionStorage.getItem('blip-coins') === '2');
});
