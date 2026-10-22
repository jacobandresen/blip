// The arcade aisle: approach a machine, insert a coin, start its game, and use its controls.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

const GAMES = [
  ['rally', 'RALLY', 2],
  ['bouncer', 'BOUNCER', 1],
  ['galactic_defender', 'DEFENDER', 1],
  ['bubbler', 'BUBBLER', 2],
  ['sky_raider', 'RAIDER', 1],
  ['meteors', 'METEORS', 1],
  ['serpent', 'SERPENT', 1],
  ['brawler', 'BRAWLER', 2],
  ['adder', 'ADDER', 2],
];

async function openArcade(t, width = 390, height = 844) {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: width < 700, viewport: { width, height },
  });
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
  return { page, origin };
}

const selectedSlug = (page) => page.locator('.card-focused').getAttribute('data-game-url');

test('the aisle shows one complete, selectable cabinet for every game', async (t) => {
  const { page } = await openArcade(t, 1280);
  const cabinets = await page.locator('#game-grid .card').evaluateAll((cards) => cards.map((card) => ({
    slug: card.dataset.gameUrl.split('/')[0],
    title: card.querySelector('.card-title')?.textContent.trim(),
    stations: card.querySelectorAll('.machine-station').length,
    hasScreen: card.querySelector('.card-shot')?.complete && card.querySelector('.card-shot')?.naturalWidth > 0,
    hasCoinDoor: !!card.querySelector('.machine-coinbox .machine-coin-slot'),
    hasStart: !!card.querySelector('.machine-start'),
    deck: card.querySelector('.machine-controls iframe')?.getAttribute('src'),
  })));
  assert.deepEqual(cabinets, GAMES.map(([slug, title, stations]) => ({
    slug, title, stations, hasScreen: true, hasCoinDoor: true, hasStart: true, deck: `machine-deck.html?game=${slug}`,
  })));
  const marqueeArt = await page.locator('#game-grid .card').evaluateAll((cards) =>
    cards.map((card) => getComputedStyle(card).getPropertyValue('--marquee-art').trim()));
  assert.equal(new Set(marqueeArt).size, GAMES.length, 'each machine has its own printed marquee motif');
  assert.ok(marqueeArt.every((art) => art.includes('gradient')));
  assert.equal(await page.locator('#kiosk-bar').isVisible(), false, 'the controls belong to the machines');
  assert.equal(await page.locator('#kiosk-insert-btn').isVisible(), false, 'the coin doors belong to the machines');
  assert.equal(await page.locator('#card-button,#controls-card').count(), 0, 'no "?" and no operator’s card on the arcade');
  assert.equal(await page.locator('.jukebox-storage,.jukebox-module').count(), 0,
    'the old selector rack is gone');
  const aisle = await page.evaluate(() => ['adder', 'rally', 'bouncer'].map((slug) => {
    const r = document.querySelector(`.card-${slug}`).getBoundingClientRect();
    return r.left + r.width / 2;
  }));
  assert.ok(aisle[0] < aisle[1] && aisle[1] < aisle[2],
    'the first machine has a neighbor on each side of the loop');
});

for (const [width, height] of [[1280, 844], [390, 844], [320, 568]]) {
  test(`the selected machine and its controls fit the arcade at ${width}×${height}`, async (t) => {
    const { page } = await openArcade(t, width, height);
    const fit = await page.evaluate(() => {
      const frame = document.querySelector('.screen-bezel').getBoundingClientRect();
      const card = document.querySelector('.card-focused');
      const controls = card.querySelector('.machine-controls').getBoundingClientRect();
      const coin = card.querySelector('.machine-coinbox').getBoundingClientRect();
      const start = card.querySelector('.machine-start').getBoundingClientRect();
      return {
        frameHeight: frame.height,
        startHeight: start.height,
        controlsInside: controls.left >= frame.left && controls.right <= frame.right && controls.bottom <= frame.bottom,
        coinInside: coin.left >= frame.left && coin.right <= frame.right && coin.bottom <= frame.bottom,
        startBelowDeck: start.top >= controls.bottom - 1 && start.bottom <= coin.top + 1 && start.bottom <= frame.bottom,
        overflow: document.documentElement.scrollWidth - innerWidth,
      };
    });
    assert.ok(fit.frameHeight > height * .6, 'the old shared deck has not collapsed the arcade frame');
    if(width < 700)assert.ok(fit.startHeight >= 30, 'the Start button has a usable touch target');
    assert.deepEqual({ ...fit, frameHeight: undefined, startHeight: undefined }, {
      frameHeight: undefined, startHeight: undefined, controlsInside: true, coinInside: true, startBelowDeck: true, overflow: 0,
    });
  });
}

test('arrows and a tap walk between machines; tapping the cabinet body does not launch it', async (t) => {
  const { page, origin } = await openArcade(t, 390);
  assert.equal(await selectedSlug(page), 'rally/index.html');
  await page.locator('.pick-next').tap();
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
  assert.equal(await page.locator('.card-shuffling').count(), 0, 'machines slide past instead of shuffling');
  await page.locator('.pick-prev').tap();
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'rally/index.html');
  await page.locator('.card-rally').tap();
  assert.equal(new URL(page.url()).pathname, '/index.html');
  assert.equal(await selectedSlug(page), 'rally/index.html');
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
  assert.equal(page.url(), `${origin}/index.html`);
});

test('returning from a game restores its machine without loading a hidden second game', async (t) => {
  const { page, origin } = await openArcade(t);
  await page.locator('.card-focused .machine-coinbox').tap();
  await page.locator('.card-focused .machine-start').tap();
  await page.waitForURL(`${origin}/rally/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.locator('.blip-logo').click();
  await page.waitForURL(`${origin}/index.html`);
  await page.waitForFunction(() => !window.blipCabinetReturn &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
  assert.equal(await selectedSlug(page), 'rally/index.html');
  assert.equal(await page.locator('#cabinet-game-screen').count(), 0);
  assert.equal(await page.evaluate(() => sessionStorage.getItem('blip-loaded-card')), null);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
});

test('a tap on the chosen machine’s screen starts it, paying its own coin', async (t) => {
  const { page, origin } = await openArcade(t, 390);
  assert.equal(await page.locator('.card-focused .machine-attract b').textContent(), 'PRESS START');
  await page.locator('.card-focused .card-shot-wrap').tap({ force: true });
  await page.waitForURL(`${origin}/rally/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.waitForFunction(() => getCoins() === 0);
});

test('desktop keyboard walking and physical Start begin play at the chosen machine', async (t) => {
  const { page, origin } = await openArcade(t, 1280);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
  await page.locator('.card-focused .machine-coinbox').click();
  await page.locator('.card-focused .machine-start').click();
  await page.waitForURL(`${origin}/bouncer/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.waitForFunction(() => getCoins() === 0);
  assert.equal(await page.evaluate(() => document.activeElement?.tagName), 'CANVAS');
  await page.evaluate(() => {
    window.__cabinetKeys = [];
    document.querySelector('canvas').addEventListener('keydown', (event) => window.__cabinetKeys.push(event.code));
  });
  await page.keyboard.press('ArrowLeft');
  await page.keyboard.press('Space');
  assert.deepEqual(await page.evaluate(() => window.__cabinetKeys.slice(-2)), ['ArrowLeft', 'Space']);
});

test('keyboard coin input updates the selected door and its Start lamp', async (t) => {
  const { page } = await openArcade(t, 1280);
  const readout = page.locator('.card-focused .machine-credit');
  assert.equal(await readout.textContent(), 'CREDIT 0');
  await page.keyboard.press('5');
  assert.equal(await page.evaluate(() => getCoins()), 1);
  assert.equal(await readout.textContent(), 'CREDIT 1');
  assert.equal(await page.locator('.card-focused').evaluate((card) => card.classList.contains('card-ready')), true);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
  assert.equal(await page.locator('.card-focused .machine-credit').textContent(), 'CREDIT 1');
});

test('a full credit bank refuses another coin without showing a coin drop', async (t) => {
  const { page } = await openArcade(t, 1280);
  for (let coin = 0; coin < 5; coin++) await page.keyboard.press('5');
  assert.equal(await page.evaluate(() => getCoins()), 5);
  const door = page.locator('.card-focused .machine-coinbox');
  await door.click();
  assert.equal(await page.evaluate(() => getCoins()), 5);
  assert.equal(await page.locator('.card-focused .machine-credit').textContent(), 'CREDIT 5');
  assert.equal(await door.evaluate((el) => el.classList.contains('coin-inserting')), false);
});

test('gamepad Select inserts a coin at the machine, then Start launches it', async (t) => {
  const { page, origin } = await openArcade(t, 1280);
  await page.evaluate(() => {
    const buttons = Array.from({ length: 16 }, () => ({ pressed: false, value: 0 }));
    window.__cabinetPadButtons = buttons;
    navigator.getGamepads = () => [{ connected: true, buttons, axes: [0, 0] }];
  });
  await page.evaluate(() => { window.__cabinetPadButtons[8].pressed = true; });
  await page.waitForFunction(() => getCoins() === 1);
  assert.equal(await page.locator('.card-focused .machine-credit').textContent(), 'CREDIT 1');
  await page.evaluate(() => { window.__cabinetPadButtons[8].pressed = false; });
  await page.evaluate(() => { window.__cabinetPadButtons[9].pressed = true; });
  await page.waitForURL(`${origin}/rally/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.waitForFunction(() => getCoins() === 0);
});

for (const width of [1280, 390, 320]) {
  test(`walking the entire aisle keeps each cabinet in view at ${width}px`, async (t) => {
    const { page } = await openArcade(t, width, width === 320 ? 568 : 844);
    for (let stop = 1; stop <= GAMES.length; stop++) {
      await page.keyboard.press('ArrowRight');
      const next = GAMES[stop % GAMES.length][0];
      await page.waitForFunction((slug) => {
        const chosen = document.querySelector('.card-focused');
        return chosen?.dataset.gameUrl === `${slug}/index.html`;
      }, next);
      await page.waitForFunction(() => {
        const frame = document.querySelector('.screen-bezel').getBoundingClientRect();
        const cabinet = document.querySelector('.card-focused').getBoundingClientRect();
        return cabinet.left >= frame.left - 1 && cabinet.right <= frame.right + 1 && cabinet.bottom <= frame.bottom + 1;
      });
      assert.equal(await selectedSlug(page), `${next}/index.html`);
    }
  });
}

for (const [index, [slug, title, players]] of GAMES.entries()) {
  test(`${title}: press its Start and reach playable controls`, async (t) => {
    const { page, origin } = await openArcade(t);
    for (let step = 0; step < index; step++) await page.keyboard.press('ArrowRight');
    await page.waitForFunction((game) => document.querySelector('.card-focused')?.dataset.gameUrl === `${game}/index.html`, slug);
    const machine = page.locator('.card-focused');
    assert.equal(await machine.locator('.machine-station').count(), players);
    assert.equal(await page.evaluate(() => getCoins()), 0);

    if (index % 2) {
      await machine.locator('.machine-coinbox').tap();
      assert.equal(await page.evaluate(() => getCoins()), 1, 'the machine coin door adds one credit');
      assert.equal(await selectedSlug(page), `${slug}/index.html`, 'inserting a coin keeps the player at this machine');
      assert.equal(await machine.evaluate((card) => card.classList.contains('card-ready')), true);
    }
    // Without a credit, Start drops the coin itself.
    await machine.locator('.machine-start').tap();
    await page.waitForURL(`${origin}/${slug}/index.html`, { timeout: 30000 });
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), null, { timeout: 30000 });
    const game = await page.evaluate(() => {
      const visible = (selector) => [...document.querySelectorAll(selector)].some((el) => {
        const r = el.getBoundingClientRect(), style = getComputedStyle(el);
        return r.width >= 10 && r.height >= 10 && style.display !== 'none' && style.visibility !== 'hidden';
      });
      return {
        slug: blipGameFromPath(location.pathname)?.slug,
        credit: getCoins(),
        returnTo: blipFrontendUrl(),
        controls: BLIP_GAMES[blipGameFromPath(location.pathname)?.slug]?.touchControls?.length || 0,
        screen: visible('canvas'),
        deck: visible('#topbar .deck-panel'),
        move: visible('#stick-base,#snes-pad .snes-dpad,#paddle-dial,#touch-strip'),
        action: visible('#fire-buttons .arcade-btn,#snes-pad .snes-face,#paddle-dial-p2,#touch-strip'),
      };
    });
    assert.equal(game.slug, slug);
    assert.ok(game.credit === 0 || game.credit === 1, `${title} kept or spent the inserted credit`);
    assert.equal(game.returnTo, '../index.html', `${title} returns to the arcade`);
    assert.ok(game.controls > 0, `${title} has a touch control guide`);
    assert.deepEqual([game.screen, game.deck, game.move, game.action], [true, true, true, true],
      `${title} started with a screen and usable controls: ${JSON.stringify(game)}`);
    await page.waitForFunction(() => getCoins() === 0, null, { timeout: 5000 });
    assert.equal(await page.evaluate(() => sessionStorage.getItem('blip-cabinet-start')), null,
      `${title} consumed the cabinet Start action`);
    if (slug === 'rally') {
      const touchState = await page.locator('#touch-strip .ts-glass').evaluate((surface) => {
        const box = surface.getBoundingClientRect();
        const details = { bubbles: true, pointerId: 71, pointerType: 'touch',
          clientX: box.left + box.width / 4, clientY: box.top + box.height / 2 };
        surface.dispatchEvent(new PointerEvent('pointerdown', details));
        const held = window.blipTouchDown(0);
        surface.dispatchEvent(new PointerEvent('pointerup', details));
        return held;
      });
      assert.equal(touchState, 2, 'Rally’s paddle strip reaches the game as a pressed touch');
    } else {
      await page.evaluate(() => {
        window.__cabinetActionKeys = [];
        document.querySelector('canvas').addEventListener('keydown', (event) => window.__cabinetActionKeys.push(event.code));
      });
      const actionTarget = ['serpent', 'bouncer', 'galactic_defender', 'bubbler', 'adder'].includes(slug) ?
        '#touch-strip .ts-glass' : '#snes-pad .snes-btn[data-blip="button1"]';
      const action = page.locator(actionTarget);
      if (slug === 'bubbler') {
        const box = await action.boundingBox();
        await action.tap({ timeout: 3000, position: { x: box.width * .9, y: box.height * .8 } });
      } else await action.tap({ timeout: 3000 });
      const expected = await page.evaluate(() => BLIP_GAMES[blipGameFromPath(location.pathname).slug]?.buttons?.[0]?.code || 'Space');
      await page.waitForFunction((code) => window.__cabinetActionKeys?.includes(code), expected, { timeout: 2000 });
    }
  });
}

test('a game opened in the arcade after a visit to BASIC returns to the arcade from the BLIP badge', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 900 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => localStorage.setItem('blip-frontend', 'basic'));
  await page.goto(`${origin}/index.html?cabinet`);
  await page.waitForFunction(() => !document.documentElement.hasAttribute('data-cabinet-arranging'));
  await page.goto(`${origin}/serpent/index.html`);
  await page.locator('.blip-logo').click();
  await page.waitForURL((url) => !url.pathname.includes('/serpent/'));
  assert.equal(new URL(page.url()).pathname, '/index.html', 'the badge leads back to the arcade, not to BASIC');
  await page.waitForTimeout(500);
  assert.equal(new URL(page.url()).pathname, '/index.html', 'and the arcade stays');
});
