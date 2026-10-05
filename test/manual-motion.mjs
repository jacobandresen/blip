import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const width of [1280, 390]) {
  test(`field manual returns to its upper-right pocket at ${width}px`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width, height: 900 } });
    await page.goto(origin + '/index.html');
    const book = page.locator('#manual-book');
    const slot = await book.boundingBox();
    assert.ok(slot.x > width / 2, 'the book is stored on the right side of the fascia');
    await book.click();
    await page.waitForSelector('.manual-transit');
    const start = await page.locator('.manual-transit').evaluate((el) => el.getAnimations()[0].effect.getKeyframes()[0]);
    assert.ok(Math.abs(parseFloat(start.left) - slot.x) < 1);
    assert.ok(Math.abs(parseFloat(start.top) - slot.y) < 1);
    await page.waitForFunction(() => document.querySelector('.manual-open-book').getAnimations()
      .some((a) => a.effect.getKeyframes()[0].transform?.startsWith('scaleX(')));
    const handoff = await page.evaluate(() => {
      const cover = document.querySelector('.manual-transit').getBoundingClientRect();
      const pages = document.querySelector('.manual-open-book').getBoundingClientRect();
      return ['left', 'top', 'width', 'height'].map((key) => Math.abs(cover[key] - pages[key]));
    });
    assert.ok(handoff.every((difference) => difference < 2),
      `cover and pages change size at the handoff: ${JSON.stringify(handoff)}`);
    await page.waitForFunction(() => !document.getElementById('manual-overlay').inert);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'hidden');
    await page.getByRole('button', { name: 'Close field manual', exact: true }).click();
    await page.waitForSelector('.manual-transit');
    const end = await page.locator('.manual-transit').evaluate((el) => el.getAnimations()[0].effect.getKeyframes().at(-1));
    assert.ok(Math.abs(parseFloat(end.left) - slot.x) < 1);
    assert.ok(Math.abs(parseFloat(end.top) - slot.y) < 1);
    await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'visible');
    assert.equal(await book.evaluate((el) => document.activeElement === el), true);
    assert.equal(await page.locator('.manual-transit').count(), 0);

    await page.emulateMedia({ reducedMotion: 'reduce' });
    await book.click();
    assert.equal(await page.locator('.manual-transit').count(), 0);
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'visible');
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await book.click();
    await page.keyboard.press('Escape');
    await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'visible');
    assert.equal(await page.locator('.manual-transit').count(), 0);
  });
}
