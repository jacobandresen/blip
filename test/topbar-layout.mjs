// The logo belongs on the marquee; the coinbox belongs on the shared control
// deck. Check both physical placements across every page and viewport.
// Animations are disabled first: the logo's boot flourish and the slot's
// beckon scale their boxes by a few percent.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, existsSync } from 'node:fs';
import path from 'node:path';
import { openPage, HTTP_PORT, WEB_DIR, evaluate, waitFor, sleep } from './lib/harness.mjs';


/** Every built game page, discovered rather than listed, so a new game
 * is covered the day it is added instead of the day someone remembers. */
const GAME_PAGES = readdirSync(WEB_DIR, { withFileTypes: true })
  .filter((d) => d.isDirectory() && existsSync(path.join(WEB_DIR, d.name, 'index.html')))
  .map((d) => `${d.name}/index.html`);

const KIOSK_PAGES = ['index.html', 'about.html', 'history.html', 'controls.html']
  .filter((f) => existsSync(path.join(WEB_DIR, f)));

const VIEWPORTS = [
  { name: 'desktop', width: 1280, height: 800 },
  { name: 'narrow desktop', width: 768, height: 600 },
  { name: 'phone portrait', width: 390, height: 700 },
  { name: 'small phone', width: 320, height: 568 },
  { name: 'phone landscape (short)', width: 700, height: 360 },
];

const FREEZE_ANIMATIONS = `(function () {
  var s = document.createElement('style');
  s.textContent = '*, *::before, *::after { animation: none !important; transition: none !important; }';
  document.head.appendChild(s);
  return true;
})()`;

/** Geometry of the marquee and the shared bottom deck. */
const MEASURE = `(function () {
  function box(el) {
    if (!el) return null;
    var b = el.getBoundingClientRect();
    return { top: b.top, bottom: b.bottom, left: b.left, right: b.right, h: b.height, w: b.width };
  }
  var bar = document.querySelector('#marquee-bar') || document.querySelector('.top-marquee-bar');
  var logo = document.querySelector('.blip-logo');
  var coin = document.querySelector('#insert-coin-btn') || document.querySelector('#kiosk-insert-btn');
  return {
    bar: box(bar), logo: box(logo), coin: box(coin),
    coinBar: box(coin && coin.closest('.kiosk-bar')),
    coinPlane: coin && getComputedStyle(coin).transform,
    deckPlane: coin && getComputedStyle(coin.closest('.kiosk-bar').querySelector('.deck-panel')).transform,
    winW: window.innerWidth,
    scrollW: document.documentElement.scrollWidth,
  };
})()`;

// A pixel of slack: sub-pixel rounding at fractional device ratios, and
// the strip's own 1px bottom border, are not "hanging out of the bar".
const SLACK = 1.5;

function assertInsideBar(m, label) {
  assert.ok(m.bar, `${label}: no top bar found`);
  assert.ok(m.logo, `${label}: no BLIP logo found`);
  assert.ok(m.coin, `${label}: no COINS button found`);

  for (const [name, el] of [['BLIP logo', m.logo]]) {
    assert.ok(el.bottom <= m.bar.bottom + SLACK,
      `${label}: the ${name} hangs ${(el.bottom - m.bar.bottom).toFixed(1)}px below the bar ` +
      `(bar ${m.bar.h.toFixed(0)}px tall, ${name} ${el.h.toFixed(0)}px)`);
    assert.ok(el.top >= m.bar.top - SLACK,
      `${label}: the ${name} sticks ${(m.bar.top - el.top).toFixed(1)}px above the bar`);

    // Horizontally inside the viewport too — an element pushed off the
    // right edge is just as gone as one hanging below the bar.
    assert.ok(el.left >= -SLACK, `${label}: the ${name} starts off the left edge (${el.left.toFixed(1)}px)`);
    assert.ok(el.right <= m.winW + SLACK,
      `${label}: the ${name} runs ${(el.right - m.winW).toFixed(1)}px past the right edge`);
  }

  assert.ok(m.coinBar, `${label}: the coinbox is not on the shared deck`);
  assert.ok(m.coin.left >= -SLACK && m.coin.right <= m.winW + SLACK,
    `${label}: the coinbox runs outside the viewport`);
  assert.equal(m.coinPlane, m.deckPlane,
    `${label}: the coinbox and control deck do not share a tabletop plane`);

}

/** Page-wide horizontal overflow, reported apart from the bar: a real defect,
 * but not this test's subject. */
test(`the BLIP logo stays on the marquee and coinbox aligns with the deck`, async (t) => {
  const { browser, page, cdp } = await openPage(t);

  assert.ok(GAME_PAGES.length > 0, 'found no built game pages to check');

  for (const vp of VIEWPORTS) {
    for (const rel of [...GAME_PAGES, ...KIOSK_PAGES]) {
      await t.test(`${rel} at ${vp.name} ${vp.width}x${vp.height}`, async () => {
        await page.setViewportSize({ width: vp.width, height: vp.height });
        await cdp.send('Page.navigate', { url: `http://127.0.0.1:${HTTP_PORT}/${rel}` });
        await waitFor(cdp, "document.readyState === 'complete'", 20000);
        await evaluate(cdp, FREEZE_ANIMATIONS);
        // The game pages build their marquee from shell.js after load.
        await waitFor(cdp, `!!(document.querySelector('#marquee-bar') || document.querySelector('.top-marquee-bar'))`, 15000);
        await waitFor(cdp, `!!document.querySelector('.deck-coin-slot .coin-plate')`, 15000);
        await sleep(120); // let the strip's own layout settle before measuring

        const m = await evaluate(cdp, MEASURE);
        assertInsideBar(m, `${rel} @ ${vp.width}x${vp.height}`);
      });
    }
  }
});
