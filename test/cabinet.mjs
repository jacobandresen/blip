// The cabinet's front page: the card rack, its shuffle, touch, and the coin.
//
//   node --test test/cabinet.mjs        (after ./build_web.sh)

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

const GAMES = ['RALLY', 'BOUNCER', 'DEFENDER', 'BUBBLER', 'RAIDER', 'METEORS', 'SERPENT', 'BRAWLER'];

async function openCabinet(t, { width = 1280, height = 844, touch = false } = {}) {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: touch, viewport: { width, height } });
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused')?.offsetWidth > 0);
  return { page, origin };
}

/** Resolves once no card is mid-move, whether by transition or animation. */
const settled = (page) => page.waitForFunction(() =>
  [...document.querySelectorAll('.card')].every((card) => card.getAnimations().length === 0));

const focusedTitle = (page) => page.locator('.card-focused .card-title').textContent();

for (const width of [1280, 768, 390, 320]) {
  test(`every card stays inside the cabinet with readable text at ${width}px`, async (t) => {
    const { page } = await openCabinet(t, { width });
    for (const title of GAMES) {
      await settled(page);
      assert.equal(await focusedTitle(page), title);
      const fit = await page.evaluate(() => {
        const card = document.querySelector('.card-focused');
        const box = card.getBoundingClientRect();
        const bezel = document.querySelector('.screen-bezel').getBoundingClientRect();
        const text = ['.card-title', '.card-genre', '.card-desc'].flatMap((selector) => {
          const range = document.createRange();
          range.selectNodeContents(card.querySelector(selector));
          return [...range.getClientRects()].filter((rect) => rect.width > 0);
        });
        return {
          inBezel: box.left >= bezel.left && box.right <= bezel.right && box.bottom <= bezel.bottom,
          textOnCard: text.every((r) => r.left >= box.left && r.right <= box.right && r.bottom <= box.bottom),
          fontPx: parseFloat(getComputedStyle(card.querySelector('.card-desc')).fontSize),
          pageOverflow: document.documentElement.scrollWidth - innerWidth,
        };
      });
      assert.deepEqual({ ...fit, fontPx: fit.fontPx >= 14 },
        { inBezel: true, textOnCard: true, fontPx: true, pageOverflow: 0 }, `${title}`);
      await page.keyboard.press('ArrowRight');
    }
  });
}

test('the card fanned out in the rack never leaves the cabinet on a phone', async (t) => {
  const { page } = await openCabinet(t, { width: 320 });
  const outside = await page.evaluate(() => {
    const bezel = document.querySelector('.screen-bezel').getBoundingClientRect();
    return [...document.querySelectorAll('.card')].filter((card) => {
      const box = card.getBoundingClientRect();
      return box.left < bezel.left || box.right > bezel.right;
    }).length;
  });
  assert.equal(outside, 0);
});

test('the arrow keys step through the rack, and the card that leaves tucks in behind the new front', async (t) => {
  const { page } = await openCabinet(t);
  assert.equal(await focusedTitle(page), 'RALLY');
  await page.keyboard.press('ArrowRight');
  await page.waitForTimeout(100);
  const leaving = await page.evaluate(() => {
    const card = document.querySelector('.card-rally');
    return { animated: card.getAnimations().length > 0, shuffling: card.classList.contains('card-shuffling') };
  });
  assert.deepEqual(leaving, { animated: true, shuffling: true }, 'RALLY is lifted out while BOUNCER comes forward');
  await settled(page);
  const stack = await page.evaluate(() => ({
    rally: Number(document.querySelector('.card-rally').style.zIndex),
    bouncer: Number(document.querySelector('.card-bouncer').style.zIndex),
    shuffling: document.querySelector('.card-shuffling') !== null,
  }));
  assert.equal(await focusedTitle(page), 'BOUNCER');
  assert.ok(stack.rally < stack.bouncer, 'RALLY rests behind BOUNCER');
  assert.equal(stack.shuffling, false);

  await page.keyboard.press('ArrowLeft');
  await settled(page);
  assert.equal(await focusedTitle(page), 'RALLY');
  await page.keyboard.press('ArrowLeft');
  await settled(page);
  assert.equal(await page.evaluate(() => document.getElementById('manual-book').classList.contains('card-focused')),
    true, 'the rack wraps round to the field manual, the last item');
});

test('a swipe steps the rack either way, and a long one steps across several cards', async (t) => {
  const { page } = await openCabinet(t, { width: 390, touch: true });
  const box = await page.locator('#game-grid').boundingBox();
  const y = box.y + box.height / 2;
  const touch = await page.context().newCDPSession(page);
  const swipe = async (from, to) => {
    const at = (fraction) => box.x + box.width * fraction;
    await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: at(from), y }] });
    await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ id: 1, x: at(to), y }] });
    await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    await page.waitForTimeout(1200);
  };
  const first = await focusedTitle(page);
  await swipe(0.78, 0.22);
  const next = await focusedTitle(page);
  assert.notEqual(next, first, 'a left swipe moves on');
  await swipe(0.22, 0.78);
  assert.equal(await focusedTitle(page), first, 'a right swipe comes back');
  await swipe(0.84, 0.16);
  await page.waitForTimeout(1500);
  assert.ok(GAMES.indexOf(await focusedTitle(page)) >= 2, 'a long swipe steps across more than one card');
});

test('a coin lights the cards; a first tap arms a card and a second opens the game with its credit', async (t) => {
  const { page } = await openCabinet(t, { width: 390, touch: true });
  const lit = () => page.evaluate(() => !document.querySelector('.card-bouncer').classList.contains('no-credit'));
  assert.equal(await page.evaluate(() => getCoins()), 0);
  assert.equal(await lit(), false, 'the cards are dark with no credit');

  await page.locator('#kiosk-insert-btn').tap();
  assert.equal(await page.evaluate(() => getCoins()), 1);
  await page.waitForFunction(() => !document.querySelector('.card-bouncer').classList.contains('no-credit'));

  const box = await page.locator('#game-grid').boundingBox();
  const y = box.y + box.height / 2;
  const touch = await page.context().newCDPSession(page);
  await touch.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: box.x + box.width - 24, y }] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ id: 1, x: box.x + box.width - 150, y }] });
  await touch.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await page.waitForFunction(() => document.querySelector('.card-bouncer').classList.contains('card-focused'));

  await page.locator('.card-bouncer').tap();
  assert.equal(await page.evaluate(() => location.pathname), '/index.html', 'the first tap only arms the card');
  await page.locator('.card-bouncer').tap();
  await page.waitForURL(/\/bouncer\/index\.html/, { timeout: 25000 });
  assert.equal(await page.evaluate(() => getCoins()), 1, 'the credit reaches the game');
});

test('the coin slot takes a touch in every layout, and its box sits flush against the deck', async (t) => {
  const { page, origin } = await openCabinet(t, { width: 390, touch: true });
  const flush = () => page.evaluate(() => {
    const slot = document.querySelector('.deck-coin-slot');
    const deck = slot.closest('.kiosk-bar').querySelector('.deck-panel');
    const label = slot.querySelector('.coin-instruction');
    const mouth = slot.querySelector('.coin-mouth').getBoundingClientRect();
    return {
      gap: Math.abs(deck.offsetLeft + deck.offsetWidth - slot.offsetLeft),
      labelFits: label.scrollWidth <= label.clientWidth,
      slotWide: mouth.width > mouth.height * 4,
    };
  });
  const insert = async (expected) => {
    await page.locator('.deck-coin-slot').tap();
    assert.equal(await page.evaluate(() => getCoins()), expected);
  };
  const expectFlush = async (where) => {
    const state = await flush();
    assert.ok(state.gap <= 1 && state.labelFits && state.slotWide, `${where}: ${JSON.stringify(state)}`);
  };

  await expectFlush('upright');
  await insert(1);

  await page.setViewportSize({ width: 844, height: 390 });
  await page.waitForFunction(() => document.documentElement.dataset.layout === 'landscape');
  await expectFlush('sideways');
  await insert(2);

  await page.setViewportSize({ width: 320, height: 568 });
  await page.waitForFunction(() => document.documentElement.dataset.layout === 'upright');
  await expectFlush('narrow');
  await insert(3);

  await page.goto(`${origin}/bouncer/index.html`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.deck-coin-slot');
  await expectFlush('game page');
  await insert(4);
});

/** Pauses each arm cycle at time zero so a test can step it to any millisecond. */
const stepTransfers = (page) => page.evaluate(() => {
  const real = blipTransferCard;
  window.__cycles = [];
  window.blipTransferCard = (options) => {
    if (!options.returning) options.onDone = () => {};
    const stage = real(options);
    if (!options.idle) { stage.blipSeek(0); window.__cycles.push({ stage, options }); }
    return stage;
  };
});
const seek = (page, cycle, ms) => page.evaluate(([c, t]) => window.__cycles[c].stage.blipSeek(t), [cycle, ms]);
const title = (page) => page.locator('#marquee-name').getAttribute('aria-label');

test('choosing a game sends the loaded card back, then seats the new one before its game starts', async (t) => {
  const { page } = await openCabinet(t);
  await page.evaluate(() => {
    saveCoins(3);
    loadedCard = { slug: 'bouncer', name: 'BOUNCER', code: 'A2', art: 'bouncer/screenshot.png' };
    document.querySelector('.card-bouncer').classList.add('card-loaded');
    blipSetMarquee('BOUNCER', false);
  });
  await stepTransfers(page);
  await page.evaluate(() => document.querySelector('.card-galactic').click());
  await page.waitForFunction(() => window.__cycles.length === 1);

  assert.equal(await page.evaluate(() => window.__cycles[0].options.returning), true, 'the loaded card goes back first');
  await seek(page, 0, 3000);
  assert.equal(await title(page), 'BOUNCER', 'the old title stays until its card is released');
  await seek(page, 0, 4600);
  assert.equal(await title(page), 'No game selected');
  assert.equal(await page.locator('#cabinet-game-screen').getAttribute('src'), 'about:blank');
  await seek(page, 0, 9600);
  assert.equal(await page.locator('.card-bouncer .game-rom').evaluate((el) => getComputedStyle(el).visibility), 'visible',
    'the old cartridge is back on its card');
  await page.evaluate(() => { window.__cycles[0].stage.remove(); window.__cycles[0].options.onDone(); });

  await page.waitForFunction(() => window.__cycles.length === 2);
  assert.equal(await page.evaluate(() => !!window.__cycles[1].options.returning), false, 'then the new card goes in');
  const fascia = () => page.evaluate(() => document.querySelector('.top-marquee-bar').getBoundingClientRect().bottom);
  await seek(page, 1, 2100);
  for (let ms = 5200; ms <= 9400; ms += 200) {
    await seek(page, 1, ms);
    const pivot = await page.evaluate(() => parseFloat(window.__cycles[1].stage.querySelector('.jukebox-arm-pivot').style.top));
    assert.ok(pivot >= await fascia(), `the arm's pivot stays below the fascia at ${ms}ms`);
  }
  await seek(page, 1, 9590);
  assert.equal(await page.locator('#cabinet-game-screen').getAttribute('src'), 'about:blank', 'the game waits for the card to seat');
  await seek(page, 1, 9600);
  await page.waitForFunction(() => document.getElementById('cabinet-game-screen').src.endsWith('/galactic_defender/index.html?cabinet=1'));
  assert.equal(await title(page), 'DEFENDER');
});

test('a game whose card is already seated opens at once, with the arm still', async (t) => {
  const { page, origin } = await openCabinet(t);
  await page.evaluate(() => sessionStorage.setItem('blip-loaded-card',
    JSON.stringify({ slug: 'bouncer', name: 'BOUNCER', code: 'A2', art: 'bouncer/screenshot.png' })));
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-bouncer')?.classList.contains('card-loaded'));
  await page.evaluate(() => {
    saveCoins(3);
    document.querySelector('.card-bouncer').click();
  });
  await page.waitForURL(/\/bouncer\/index\.html/, { timeout: 20000 });
  assert.equal(await page.evaluate(() => document.querySelectorAll('.jukebox-transfer:not(.idle)').length), 0,
    'no arm cycle runs on the game page');
  assert.equal(await title(page), 'BOUNCER');
});
