import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';

for (const engine of ['chromium', 'webkit']) {
  test(`fullscreen kiosk cabinets retain every physical control deck in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 1080, height: 1920 } });
    await page.addInitScript(() => localStorage.setItem('blip-fullscreen', '1'));
    await page.goto(`${origin}/index.html?room=full`);
    await page.waitForSelector('.card-focused[data-loop-ready]');
    assert.equal(await page.locator('html').getAttribute('data-fullscreen'), '');
    const cards = page.locator('#game-grid .card');
    const games = await cards.evaluateAll((cards) => cards.map((card) => card.dataset.machine));
    for (const game of games) {
      await page.waitForFunction((game) => document.querySelector('.card-focused')?.dataset.machine === game &&
        !document.getElementById('game-grid').hasAttribute('data-aisle-moving'), game);
      const deck = page.locator('.card-focused iframe').contentFrame();
      const control = deck.locator(game === 'rally' ? '#paddle-dial' : '#stick-base');
      await control.waitFor({ state: 'visible' });
      assert.equal(await deck.locator('html').getAttribute('data-fullscreen'), null, `${game}: preview retains its deck`);
      const picture = await control.screenshot();
      assert.ok(picture.length > 1000, `${game}: the physical control paints`);
      const dir = new URL('artifacts/kiosk-room/', import.meta.url);
      await mkdir(dir, { recursive: true });
      await writeFile(new URL(`${engine}-${game}.png`, dir), picture);
      await page.keyboard.press('ArrowRight');
    }
    assert.equal(await page.evaluate(() => localStorage.getItem('blip-fullscreen')), '1', 'previews preserve the kiosk preference');
  });
}

test('a kiosk keeps fullscreen through a real Start and a simulated two-minute idle return', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 800 } });
  await page.goto(`${origin}/home.html`);
  await page.locator('#dock-settings').click();
  await page.locator('#set-fullscreen').check();
  await page.waitForFunction(() => !!document.fullscreenElement);
  await page.locator('#set-cabinet').click();
  await page.waitForURL((url) => url.pathname.endsWith('/index.html'));
  await page.waitForSelector('.card-focused[data-loop-ready]');
  assert.equal(await page.locator('html').getAttribute('data-fullscreen'), '');
  await page.locator('.card-focused .machine-coinbox').click();
  await page.waitForFunction(() => !!document.fullscreenElement);
  await page.locator('.card-focused .machine-start').click();
  await page.waitForURL((url) => url.pathname.includes('/rally/'));
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  assert.equal(await page.locator('html').getAttribute('data-fullscreen'), '');
  await page.keyboard.press('ArrowUp');
  await page.waitForFunction(() => !!document.fullscreenElement);
  await page.clock.install();
  await page.clock.fastForward(125000);
  await page.waitForURL((url) => url.pathname.endsWith('/index.html') && !url.pathname.includes('/rally/'));
  await page.waitForSelector('.card-focused[data-loop-ready]');
  assert.equal(await page.locator('html').getAttribute('data-fullscreen'), '');
  assert.equal(await page.evaluate(() => localStorage.getItem('blip-fullscreen')), '1');
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => !!document.fullscreenElement);
});
