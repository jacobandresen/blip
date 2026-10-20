import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const viewport of [
  { width: 320, height: 568 },
  { width: 390, height: 844 },
  { width: 740, height: 360 },
  { width: 844, height: 390 },
]) test(`BASIC mobile layout fits ${viewport.width}×${viewport.height}`, async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport });
  await page.goto(`${origin}/basic.html`);
  await page.waitForFunction(() => document.querySelectorAll('.game-tape').length === 9);
  const layout = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    height: document.documentElement.scrollHeight,
    viewportWidth: innerWidth,
    viewportHeight: innerHeight,
    horizontal: matchMedia('(orientation:landscape) and (max-height:500px)').matches,
    guide: getComputedStyle(document.querySelector('#basic-book-peek')).display !== 'none',
  }));
  assert.equal(layout.width, layout.viewportWidth, 'the page has no sideways overflow');
  if (layout.horizontal) {
    assert.ok(layout.height <= layout.viewportHeight, 'the short landscape layout fits vertically');
    assert.equal(layout.guide, true, 'the BASIC book remains reachable');
  }
});

test('BASIC cassette strip swipes sideways on touch without selecting a tape', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: true, viewport: { width: 390, height: 844 },
  });
  await page.goto(`${origin}/basic.html`);
  await page.waitForFunction(() => document.querySelectorAll('.game-tape').length === 9);
  const tape = await page.locator('.game-tape').first().boundingBox();
  const cdp = await page.context().newCDPSession(page);
  const y = tape.y + tape.height / 2;
  await cdp.send('Input.dispatchTouchEvent', {
    type: 'touchStart', touchPoints: [{ id: 1, x: tape.x + tape.width * .8, y }],
  });
  for (const fraction of [.55, .3, .1]) await cdp.send('Input.dispatchTouchEvent', {
    type: 'touchMove', touchPoints: [{ id: 1, x: tape.x + tape.width * fraction, y }],
  });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await page.waitForFunction(() => document.querySelector('#tape-pile').scrollLeft > 50);
  assert.equal(await page.locator('body').getAttribute('data-tape-selecting'), null);
  assert.equal(await page.locator('.game-tape').count(), 9);
});

test('a running BASIC game keeps its playfield between the side controls in landscape', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: true, viewport: { width: 844, height: 390 },
  });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('10 CANVAS(320,200):CLEAR("BLIP_GREEN"):WAIT 16:GOTO 10');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-playing'));
  const game = await page.evaluate(() => {
    const rect = (selector) => {
      const { left, right, top, bottom, width, height } = document.querySelector(selector).getBoundingClientRect();
      return { left, right, top, bottom, width, height };
    };
    return {
      screen: rect('.screen'),
      stick: rect('.stick-base'),
      buttons: rect('.fire-buttons'),
      width: document.documentElement.scrollWidth,
      height: document.documentElement.scrollHeight,
    };
  });
  assert.ok(game.screen.width > 200, 'the BASIC screen stays large enough to play');
  assert.ok(game.stick.right < game.screen.left, 'the stick stays in the left gutter');
  assert.ok(game.buttons.left > game.screen.right, 'the action buttons stay in the right gutter');
  assert.deepEqual([game.width, game.height], [844, 390], 'the play view fits the landscape viewport');
});
