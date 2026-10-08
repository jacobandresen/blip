import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, HTTP_PORT } from './lib/harness.mjs';

const PHONES = [
  { name: 'iPhone 14', width: 390, height: 844 },
  { name: 'iPhone SE', width: 375, height: 667 },
];

for (const phone of PHONES) {
  for (const orientation of ['portrait', 'landscape']) {
    const viewport = orientation === 'portrait'
      ? { width: phone.width, height: phone.height }
      : { width: phone.height, height: phone.width };

    test(`Rally 2P touch controls and game stay visible on ${phone.name} ${orientation}`, async (t) => {
      const { browser, origin } = await openPage(t, 'chromium');
      const context = await browser.newContext({ viewport, hasTouch: true });
      const page = await context.newPage();
      t.after(() => context.close());
      await page.addInitScript(() => localStorage.setItem('blip-touch', '1'));
      await page.goto(`${origin}/rally/index.html`);
      await page.waitForFunction(() => document.querySelector('#glcanvas') &&
        getComputedStyle(document.querySelector('#loader')).display === 'none');
      await page.locator('#insert-coin-btn').click();
      await page.locator('#insert-coin-btn').click();
      await page.waitForFunction(() => !document.querySelector('#need-coin-overlay').classList.contains('visible'));

      const stateBefore = await page.evaluate(() => ({
        canvas: (() => { const r = document.querySelector('#glcanvas').getBoundingClientRect();
          return { x: r.x, y: r.y, width: r.width, height: r.height }; })(),
        controls: ['#touch-strip .ts-half[data-slot="0"]', '#touch-strip .ts-half[data-slot="1"]'].map((s) => {
          const r = document.querySelector(s).getBoundingClientRect();
          return { x: r.x, y: r.y, width: r.width, height: r.height,
            visible: getComputedStyle(document.querySelector(s)).display !== 'none' };
        }),
        versus: document.documentElement.hasAttribute('data-versus'),
      }));
      assert.ok(stateBefore.canvas.width > 100 && stateBefore.canvas.height > 100,
        `Rally playfield is too small: ${JSON.stringify(stateBefore.canvas)}`);
      assert.ok(stateBefore.controls.every((c) => c.visible && c.width >= 44 && c.height >= 44),
        `Rally is missing a touch-sized paddle area: ${JSON.stringify(stateBefore.controls)}`);
      const p2 = stateBefore.controls[1];
      const touch = await context.newCDPSession(page);
      await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [
        { id: 1, x: p2.x + p2.width / 2, y: p2.y + p2.height / 2 },
      ] });
      await page.waitForTimeout(200);
      const touchState = await page.evaluate(({ x, y }) => ({
        down: [window.blipTouchDown(0), window.blipTouchDown(1)],
        controls: document.documentElement.getAttribute('data-controls'),
        touch: document.documentElement.getAttribute('data-touch'),
        cpu: document.documentElement.hasAttribute('data-cpu'),
        overlay: document.querySelector('#need-coin-overlay').classList.contains('visible'),
        hit: document.elementFromPoint(x, y)?.className,
      }), { x: p2.x + p2.width / 2, y: p2.y + p2.height / 2 });
      assert.equal(touchState.down[1], 2,
        `the player-two touch area did not deliver a live finger to Rally: ${JSON.stringify(touchState)}`);
      await page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'),
        null, { timeout: 10000 }).catch(() => {});
      await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
      await touch.detach();
      assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-versus')),
        true, 'a touch on player two paddle did not enter Rally 2P');
      await page.waitForTimeout(200);
      const canvas = await page.locator('#glcanvas').boundingBox();
      assert.ok(canvas && canvas.width > 100 && canvas.height > 100,
        `Rally playfield disappeared after entering 2P: ${JSON.stringify(canvas)}`);
    });

    test(`Brawler 2P touch visibility on ${phone.name} ${orientation}`, async (t) => {
      const { browser, origin } = await openPage(t, 'chromium');
      const context = await browser.newContext({ viewport, hasTouch: true });
      const page = await context.newPage();
      t.after(() => context.close());
      await page.addInitScript(() => localStorage.setItem('blip-controls', 'pad'));
      await page.goto(`${origin}/brawler/index.html`);
      await page.waitForFunction(() => document.querySelector('#glcanvas') &&
        getComputedStyle(document.querySelector('#loader')).display === 'none');
      if (await page.locator('#need-coin-overlay.visible').count()) {
        await page.locator('#insert-coin-btn').click();
      }
      await page.waitForFunction(() => document.documentElement.hasAttribute('data-open'),
        { timeout: 20000 });
      const p2button = page.locator('#snes-pad-p2 .snes-btn').first();
      const joinTarget = await p2button.boundingBox();
      assert.ok(joinTarget, `P2 pad is hidden before joining on ${phone.name} ${orientation}`);
      const deadline = Date.now() + 5000;
      while (!await page.evaluate(() => document.documentElement.hasAttribute('data-versus')) &&
             Date.now() < deadline) {
        await page.touchscreen.tap(joinTarget.x + joinTarget.width / 2,
          joinTarget.y + joinTarget.height / 2);
        await page.waitForTimeout(250);
      }
      assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-versus')),
        true, `touching the visible player-two pad did not join ${phone.name} ${orientation}`);
      const geometry = await page.evaluate(() => {
        const rect = (el) => { const r = el.getBoundingClientRect();
          return { x: r.x, y: r.y, width: r.width, height: r.height }; };
        const canvas = rect(document.querySelector('#glcanvas'));
        const controls = Array.from(document.querySelectorAll('#snes-pad, #snes-pad-p2')).map((el) => ({
          ...rect(el), visible: getComputedStyle(el).display !== 'none',
          buttons: Array.from(el.querySelectorAll('.snes-btn')).map(rect),
          dpad: rect(el.querySelector('.snes-dpad')),
        }));
        const deck = document.querySelector('.kiosk-bar > .deck-panel');
        return { canvas, controls, versus: document.documentElement.hasAttribute('data-versus'),
          deckBackground: getComputedStyle(deck).backgroundImage,
          deckBorder: getComputedStyle(deck).borderTopWidth };
      });
      assert.ok(geometry.versus, 'Brawler did not enter 2P');
      assert.ok(geometry.canvas.width > 100 && geometry.canvas.height > 100,
        `Brawler playfield is too small: ${JSON.stringify(geometry.canvas)}`);
      assert.equal(geometry.controls.length, 2, 'both players need visible pad stations');
      for (const [i, control] of geometry.controls.entries()) {
        assert.ok(control.visible && control.buttons.length >= 2 && control.buttons.every((b) => b.width >= 32 && b.height >= 32) &&
          control.dpad.width >= 44 && control.dpad.height >= 44,
          `Brawler P${i + 1} touch station is incomplete or too small: ${JSON.stringify(control)}`);
      }
      if (orientation === 'landscape') {
        assert.equal(geometry.deckBackground, 'none', 'cabinet deck obscures the landscape game');
        assert.equal(geometry.deckBorder, '0px', 'cabinet deck border obscures the landscape game');
      }
    });
  }
}

for (const viewport of [
  { name: 'iPhone SE', width: 320, height: 568 },
  { name: 'iPhone 8', width: 375, height: 667 },
  { name: 'iPhone 14', width: 390, height: 844 },
]) {
  test(`Brawler controller selector stays clear of Manual on ${viewport.name}`, async (t) => {
    const { browser, origin } = await openPage(t, 'chromium');
    const context = await browser.newContext({ viewport, hasTouch: true });
    const page = await context.newPage();
    t.after(() => context.close());
    await page.addInitScript(() => localStorage.setItem('blip-controls', 'pad'));
    await page.goto(`${origin}/brawler/index.html`);
    await page.waitForFunction(() => document.getElementById('loader').style.display === 'none');
    if (await page.locator('#need-coin-overlay.visible').count()) {
      await page.locator('#insert-coin-btn').click();
    }
    await page.evaluate(() => window.blipSetMode(1));
    const overlap = await page.evaluate(() => {
      const picker = document.querySelector('#control-toggle');
      const manual = document.querySelector('#manual-book');
      const a = picker.getBoundingClientRect(), b = manual.getBoundingClientRect();
      return { picker: { x: a.x, y: a.y, right: a.right, bottom: a.bottom },
        manual: { x: b.x, y: b.y, right: b.right, bottom: b.bottom },
        intersects: a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top };
    });
    assert.equal(overlap.intersects, false,
      `${viewport.name}: controller selector overlaps Manual: ${JSON.stringify(overlap)}`);
    for (const mode of ['stick', 'pad']) {
      const button = page.locator(`#control-toggle [data-mode="${mode}"]`);
      assert.equal(await button.count(), 1, `${mode} selector missing`);
      const box = await button.boundingBox();
      assert.ok(box && box.width > 0 && box.height > 0, `${mode} selector is not visible`);
      await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
      assert.equal(await button.getAttribute('aria-checked'), 'true',
        `${mode} selector is obstructed or does not respond to touch`);
    }
  });
}
