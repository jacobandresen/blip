// The BLIP logo and the COINS button must stay inside the top bar.
//
// They are not decoration floating near it -- they are mounted *on* the
// marquee strip, over its two ends, and are meant to read as part of the
// same piece of cabinet trim. That only works if they are exactly as
// tall as the strip is.
//
// They had drifted apart. The strip is 28px on a desktop, 20px on a
// short screen and 44px on a phone, while both elements took a flat
// 52px -- so on a desktop the logo and COINS button hung 24px below the
// lit strip, over the game, and 32px below it on a short screen. Nothing
// caught it because the two heights were written in different files
// (web/shell.css for the elements, the same file's breakpoints for the
// strip) and no test had ever compared them.
//
// The fix is a single `--marquee-h` that all three read. This test is
// what stops them separating again: it compares measured geometry, so it
// fails whether someone changes the strip, the elements, or adds a
// breakpoint that only remembers one of them.
//
// Animations are disabled before measuring. The logo's boot flourish and
// the coin slot's beckon pulse both scale their element by a few percent,
// which is real motion but not layout -- left running they add a pixel
// or two of noise to every measurement and make the thresholds here a
// judgement call rather than a fact.

import test from 'node:test';
import assert from 'node:assert/strict';
import { readdirSync, existsSync } from 'node:fs';
import path from 'node:path';
import { openPage, HTTP_PORT, WEB_DIR, evaluate, waitFor, sleep } from './lib/harness.mjs';

const ENGINE = process.env.BLIP_TOPBAR_ENGINE || 'chromium';

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

/** Geometry of the strip and the two elements mounted on it, for
 * whichever of the two top-bar flavours the page uses. */
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

  for (const [name, el] of [['BLIP logo', m.logo], ['COINS button', m.coin]]) {
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

  assert.ok(m.logo.right <= m.coin.left + SLACK,
    `${label}: the BLIP logo and COINS button overlap by ${(m.logo.right - m.coin.left).toFixed(1)}px`);

}

/** Page-wide horizontal overflow, reported separately from the bar.
 * A sideways-scrolling page is a real defect, but it is not this test's
 * subject and can be caused by anything on the page — see the note in
 * the suite below about controls.html. */
function sidewaysOverflow(m) {
  return m.scrollW - m.winW;
}

test(`the BLIP logo and COINS button stay inside the top bar (${ENGINE})`, async (t) => {
  const { browser, page, cdp } = await openPage(t, ENGINE);

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
        await sleep(120); // let the strip's own layout settle before measuring

        const m = await evaluate(cdp, MEASURE);
        assertInsideBar(m, `${rel} @ ${vp.width}x${vp.height}`);

        // Noted, not asserted. controls.html's key-mapping table sits in
        // a wrapper wider than a phone viewport, which makes the whole
        // page scroll sideways. That predates this test and has nothing
        // to do with the top bar, whose elements are measured above and
        // are inside the bar on that page too. Surfacing it as a
        // diagnostic keeps the finding visible without making this
        // suite fail for an unrelated reason.
        const over = sidewaysOverflow(m);
        if (over > SLACK) {
          t.diagnostic(`${rel} @ ${vp.width}x${vp.height}: page scrolls sideways by ${over}px (not a top-bar fault)`);
        }
      });
    }
  }
});
