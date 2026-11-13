// Two players on one phone: each gets a touch zone and the picture stays whole, upright and sideways.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, gameReady, insertCoins } from './lib/harness.mjs';

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
      await page.goto(`${origin}/rally/index.html?controls=touch`);
      await gameReady(page);
      await insertCoins(page, 2);
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
      await page.goto(`${origin}/brawler/index.html?controls=pad`);
      await gameReady(page);
      if (await page.locator('#need-coin-overlay.visible').count()) {
        await insertCoins(page);
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

const layouts = [
  { device: 'iPhone 14', orientation: 'portrait', width: 390, height: 844 },
  { device: 'iPhone 14', orientation: 'landscape', width: 844, height: 390 },
  { device: 'iPhone SE', orientation: 'portrait', width: 375, height: 667 },
  { device: 'iPhone SE', orientation: 'landscape', width: 667, height: 375 },
  { device: 'iPhone 13 mini', orientation: 'portrait', width: 375, height: 812 },
  { device: 'iPhone 12 mini', orientation: 'landscape', width: 780, height: 360 },
];

for (const layout of layouts) {
  test(`Bubbler two-player touch layout fits ${layout.device} ${layout.orientation}`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: layout.width, height: layout.height } });
    await page.goto(`${origin}/bubbler/index.html?controls=touch`);
    await page.waitForFunction(() => document.documentElement.dataset.touch === 'platform');
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-open'));
    await insertCoins(page, 2);
    assert.equal(await page.locator('.ts-join').evaluate((button) => {
      const r = button.getBoundingClientRect();
      return document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2) === button;
    }), true, 'the 2P join button receives touches instead of the coin control');
    const joinBox = await page.locator('.ts-join').boundingBox();
    // Exercise the shell's actual two-player layout callback without making
    // the test depend on frame timing in the WASM title loop.
    await page.evaluate(() => window.blipSetMode(1));

    const geometry = await page.evaluate(() => {
      const rect = (el) => {
        const r = el.getBoundingClientRect();
        return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom };
      };
      const canvas = rect(document.querySelector('#glcanvas'));
      const deck = getComputedStyle(document.querySelector('#topbar .deck-panel'));
      const scale = Math.min(canvas.w / 624, canvas.h / 600);
      const game = {
        x: canvas.x + (canvas.w - 624 * scale) / 2,
        y: canvas.y + (canvas.h - 600 * scale) / 2,
        w: 624 * scale,
        h: 600 * scale,
      };
      const halves = Array.from(document.querySelectorAll('#touch-strip .ts-half'), rect);
      const join = rect(document.querySelector('.ts-join'));
      return {
        layout: document.documentElement.dataset.layout,
        canvas, game, halves, join,
        deckBackground: deck.backgroundImage,
        deckBorder: deck.borderTopWidth,
        screen: { w: innerWidth, h: innerHeight },
      };
    });

    assert.equal(geometry.layout, layout.orientation === 'landscape' ? 'landscape' : 'upright');
    if (layout.orientation === 'landscape') {
      assert.equal(geometry.deckBackground, 'none', 'the landscape deck stays transparent over the game');
      assert.equal(geometry.deckBorder, '0px', 'the full-screen landscape deck has no opaque panel border');
    }
    assert.equal(geometry.halves.length, 2, 'each player has a touch zone');
    assert.ok(geometry.game.w >= 250 && geometry.game.h >= 250,
      `the letterboxed game picture remains visible at a useful size: ${JSON.stringify(geometry)}`);
    assert.ok(geometry.game.x >= 0 && geometry.game.x + geometry.game.w <= geometry.screen.w + 1,
      `the game picture stays inside the phone screen: ${JSON.stringify(geometry)}`);
    assert.ok(geometry.game.y >= 0 && geometry.game.y + geometry.game.h <= geometry.screen.h + 1,
      `the game picture stays inside the phone screen: ${JSON.stringify(geometry)}`);
    for (const [i, half] of geometry.halves.entries()) {
      assert.ok(half.w >= 100 && half.h >= 100,
        `player ${i + 1} gets a touch area of at least 100×100px: ${JSON.stringify(half)}`);
    }
    assert.ok(joinBox.width >= 88 && joinBox.height >= 44,
      `the 2P join button is a usable touch target: ${JSON.stringify(joinBox)}`);
  });
}
