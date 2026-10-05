import test from 'node:test';
import assert from 'node:assert/strict';
import { startPage } from './lib/harness.mjs';

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
    const handle = await startPage('chromium', {
      hasTouch: true, viewport: { width: layout.width, height: layout.height },
    });
    t.after(handle.close);
    const { page, origin } = handle;
    await page.addInitScript(() => localStorage.setItem('blip-touch', '1'));
    await page.goto(`${origin}/bubbler/index.html`);
    await page.waitForFunction(() => document.documentElement.dataset.touch === 'platform');
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-open'));
    await page.locator('#insert-coin-btn').tap();
    await page.locator('#insert-coin-btn').tap();
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
    process.stdout.write(`${layout.device} ${layout.orientation}: game ${Math.round(geometry.game.w)}×${Math.round(geometry.game.h)} CSS px; ` +
      `touch zones ${geometry.halves.map((half) => `${Math.round(half.w)}×${Math.round(half.h)}`).join(' and ')} px\n`);
  });
}
