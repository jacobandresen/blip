import test from 'node:test';
import assert from 'node:assert/strict';
import { startPage } from './lib/harness.mjs';

test('touch can swipe to and start Bouncer', async (t) => {
  const handle = await startPage('chromium', {
    hasTouch: true, viewport: { width: 390, height: 844 },
  });
  t.after(handle.close);
  const { page, origin } = handle;
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-bouncer')?.offsetWidth > 0);
  assert.equal(await page.locator('.screen-bezel').evaluate((bezel) => bezel.classList.contains('jukebox-open')), false,
    'the Rolodex is deployed on the cabinet front page');
  assert.equal(await page.locator('#jukebox-front').evaluate((rack) => getComputedStyle(rack).visibility), 'visible');
  assert.equal(await page.locator('.blip-power-indicator').count(), 1, 'one power lamp appears on the top bar');
  const assertPowerLampCentered = async (barSelector) => {
    const geometry = await page.evaluate((selector) => {
      const bar = document.querySelector(selector).getBoundingClientRect();
      const lamp = document.querySelector('.blip-power-indicator').getBoundingClientRect();
      return { bar: bar.top + bar.height / 2, lamp: lamp.top + lamp.height / 2, rightGap: bar.right - lamp.right };
    }, barSelector);
    assert.ok(Math.abs(geometry.bar - geometry.lamp) < 1, 'power lamp is vertically centered in the top bar');
    assert.ok(Math.abs(geometry.rightGap - 8) < 1, 'power lamp is aligned to the top bar right edge');
  };
  await assertPowerLampCentered('.top-marquee-bar');
  assert.equal(await page.evaluate(() => getCoins()), 0, 'the fresh cabinet starts empty');
  assert.match(await page.locator('.power-bulb').evaluate((bulb) => getComputedStyle(bulb).boxShadow),
    /inset/, 'the top bar power lamp is unlit');
  assert.equal(await page.locator('.card-bouncer').evaluate((card) => getComputedStyle(card).filter),
    'grayscale(0.72) brightness(0.52)', 'empty Rolodex cards are visibly dimmed');
  assert.deepEqual(await page.locator('.card-bouncer .rolodex-lights i').evaluateAll((lights) =>
    lights.map((light) => getComputedStyle(light).backgroundColor)),
  ['rgb(24, 26, 22)', 'rgb(24, 26, 22)', 'rgb(24, 26, 22)'], 'empty cabinet lights stay off');

  await page.evaluate(() => {
    const accepted = window.onCoinInserted;
    window.__startingCoin = false;
    window.__startupOscillators = 0;
    const create = AudioContext.prototype.createOscillator;
    AudioContext.prototype.createOscillator = function () {
      if (window.__startingCoin) window.__startupOscillators++;
      return create.call(this);
    };
    window.onCoinInserted = function () {
      window.__startingCoin = true;
      try { return accepted(); } finally { window.__startingCoin = false; }
    };
  });
  await page.locator('#kiosk-insert-btn').tap();
  assert.equal(await page.evaluate(() => getCoins()), 1, 'touch inserts a credit');
  assert.equal(await page.locator('.blip-power-indicator').count(), 1);
  assert.match(await page.locator('.power-bulb').evaluate((bulb) => getComputedStyle(bulb).boxShadow),
    /0px 0px 5px/, 'the top bar power lamp lights up');
  await page.waitForFunction(() => !document.querySelector('.card-bouncer')?.classList.contains('no-credit'));
  assert.deepEqual(await page.locator('.card-bouncer .rolodex-lights i').evaluateAll((lights) =>
    lights.map((light) => getComputedStyle(light).backgroundColor)),
  ['rgb(231, 189, 117)', 'rgb(165, 193, 139)', 'rgb(231, 189, 117)'], 'credit powers the small Rolodex lights');
  await page.waitForFunction(() => cabinetHum?.sources.length === 4 && !cabinetHum.offTimer);
  assert.equal(await page.evaluate(() => window.__startupOscillators), 1, 'coin acceptance plays the engine startup whine');

  const rack = await page.locator('#game-grid').boundingBox();
  const touch = await page.context().newCDPSession(page);
  const y = rack.y + rack.height / 2;
  await touch.send('Input.dispatchTouchEvent', {
    type: 'touchStart', touchPoints: [{ id: 1, x: rack.x + rack.width - 24, y }],
  });
  await touch.send('Input.dispatchTouchEvent', {
    type: 'touchMove', touchPoints: [{ id: 1, x: rack.x + rack.width - 150, y }],
  });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await page.waitForFunction(() => document.querySelector('.card-bouncer')?.classList.contains('card-focused'));

  const card = page.locator('.card-bouncer');
  await card.tap();
  assert.equal(await page.evaluate(() => location.pathname), '/index.html', 'first tap arms the selected card');
  await card.tap();
  await page.waitForURL(/\/bouncer\/index\.html(?:\?[^#]*)?$/, { timeout: 25000 });
  await page.locator('#glcanvas').waitFor({ state: 'visible' });
  assert.equal(await page.locator('.blip-power-indicator').count(), 1, 'the game page keeps the top bar power lamp');
  await assertPowerLampCentered('#marquee-bar');
  assert.equal(await page.evaluate(() => getCoins()), 1, 'the credit reaches the running game');
  await page.evaluate(() => blipSpendCoin());
  await page.waitForFunction(() => !document.documentElement.hasAttribute('data-credit'));
  await page.waitForFunction(() => cabinetHum === null, { timeout: 3000 });
});
