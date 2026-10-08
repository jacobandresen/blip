// The field manual: its book on the fascia, the chapters it opens to, and
// the printed leaves.
//
//   node --test test/manual.mjs        (after ./build_web.sh)

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const width of [1280, 390]) {
  test(`the book flies out of its pocket and back at ${width}px`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width, height: 900 } });
    await page.goto(`${origin}/index.html`);
    const book = page.locator('#manual-book');
    const pocket = await book.boundingBox();
    assert.ok(pocket.x > width / 2, 'the book is stored on the right of the fascia');

    const flight = (keyframe) => page.locator('.manual-transit').evaluate((el, which) => {
      const frames = el.getAnimations()[0].effect.getKeyframes();
      return frames.at(which);
    }, keyframe);
    const atPocket = (frame) =>
      Math.abs(parseFloat(frame.left) - pocket.x) < 1 && Math.abs(parseFloat(frame.top) - pocket.y) < 1;

    await book.click();
    await page.waitForSelector('.manual-transit');
    assert.ok(atPocket(await flight(0)), 'it leaves from the pocket');
    await page.waitForFunction(() => !document.getElementById('manual-overlay').inert);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'hidden', 'the pocket is empty while it is open');

    await page.getByRole('button', { name: 'Close field manual', exact: true }).click();
    await page.waitForSelector('.manual-transit');
    assert.ok(atPocket(await flight(-1)), 'it returns to the pocket');
    await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
    assert.equal(await book.evaluate((el) => getComputedStyle(el).visibility), 'visible');
    assert.equal(await book.evaluate((el) => document.activeElement === el), true, 'focus returns to the book');
    assert.equal(await page.locator('.manual-transit').count(), 0);
  });
}

test('with reduced motion the book opens and closes without a flight', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/index.html`);
  await page.locator('#manual-book').click();
  assert.equal(await page.locator('.manual-transit').count(), 0);
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.getElementById('manual-overlay').hidden);
  assert.equal(await page.locator('#manual-book').evaluate((el) => getComputedStyle(el).visibility), 'visible');
});

for (const [slug, title] of [['about', 'about blip'], ['controls', 'controls'], ['history', 'history']]) {
  test(`/${slug}.html opens the manual at its ${slug} chapter`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
    await page.goto(`${origin}/${slug}.html`);
    await page.waitForFunction(() => !document.getElementById('manual-overlay').hidden);
    assert.equal((await page.locator('#manual-book-title').textContent()).toLowerCase(), title);
  });
}

for (const [width, height] of [[1280, 844], [390, 844], [320, 720]]) {
  test(`every printed leaf fits its paper at ${width}px`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { hasTouch: width < 700, viewport: { width, height } });
    await page.goto(`${origin}/index.html?manual=about`);
    await page.waitForFunction(() => document.querySelector('.manual-page-content .manual-leaf-sheet')
      && document.getElementById('manual-page-count').textContent.split('/')[1].trim() !== '01');

    const book = await page.evaluate(() => {
      const overlay = document.getElementById('manual-overlay');
      const open = overlay.querySelector('.manual-open-book').getBoundingClientRect();
      return {
        centered: Math.abs(open.left + open.width / 2 - innerWidth / 2) < 2,
        inFront: Number(getComputedStyle(overlay).zIndex) >= 10000,
        leaves: Number(document.getElementById('manual-page-count').textContent.split('/')[1]),
      };
    });
    assert.deepEqual({ centered: book.centered, inFront: book.inFront }, { centered: true, inFront: true });
    assert.ok(book.leaves > 3, `a long chapter is split into leaves, got ${book.leaves}`);

    while (await page.locator('#manual-prev').isEnabled()) await page.locator('#manual-prev').click();

    let portraits = 0;
    for (let leaf = 1; leaf <= book.leaves; leaf++) {
      if (leaf > 1) {
        await page.locator('#manual-next').click();
        await page.waitForTimeout(100);
      }
      const state = await page.evaluate(() => {
        const content = document.querySelector('.manual-page-content');
        const sheet = content.querySelector('.manual-leaf-sheet');
        const box = sheet.getBoundingClientRect();
        const range = document.createRange();
        range.selectNodeContents(sheet);
        const text = [...range.getClientRects()].filter((r) => r.width && r.height);
        return {
          scrolls: sheet.scrollHeight > sheet.clientHeight + 1 || content.scrollHeight > content.clientHeight + 1,
          textInside: text.every((r) => r.left >= box.left - 1 && r.right <= box.right + 1 && r.bottom <= box.bottom + 1),
          portraits: [...content.querySelectorAll('.profile-avatar')].map((img) =>
            `${img.offsetWidth}x${img.offsetHeight} ${getComputedStyle(img).filter.includes('grayscale(1)')}`),
        };
      });
      assert.equal(state.scrolls, false, `leaf ${leaf} scrolls`);
      assert.equal(state.textInside, true, `leaf ${leaf} text leaves the page`);
      for (const portrait of state.portraits) assert.equal(portrait, '52x52 true', `leaf ${leaf} portrait`);
      portraits += state.portraits.length;
    }
    assert.ok(portraits > 0, 'the operators appear in the manual');
  });
}

test('a swipe across the open book turns its pages, left for next and right for previous', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  await page.goto(`${origin}/index.html?manual=about`);
  await page.waitForFunction(() => document.querySelector('.manual-page-content .manual-leaf-sheet')
    && document.getElementById('manual-page-count').textContent.split('/')[1].trim() !== '01'
    && !document.getElementById('manual-overlay').inert);
  const leaf = async () => Number((await page.locator('#manual-page-count').textContent()).split('/')[0]);
  const box = await page.locator('.manual-open-book').boundingBox();
  const y = box.y + box.height / 2;
  const swipe = async (from, to) => {
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: box.x + box.width * from, y }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ id: 1, x: box.x + box.width * to, y }] });
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [{ id: 1, x: box.x + box.width * to, y }] });
    await page.waitForTimeout(400);
  };
  const first = await leaf();
  await swipe(0.8, 0.2);
  assert.equal(await leaf(), first + 1, 'a left swipe turns to the next page');
  await swipe(0.2, 0.8);
  assert.equal(await leaf(), first, 'a right swipe turns back');
});
