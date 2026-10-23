// The arcade aisle: approach a machine, insert a coin, start its game, and use its controls.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

const GAMES = [
  ['rally', 'RALLY'],
  ['bouncer', 'BOUNCER'],
  ['defender', 'DEFENDER'],
  ['bubbler', 'BUBBLER'],
  ['raider', 'RAIDER'],
  ['meteors', 'METEORS'],
  ['serpent', 'SERPENT'],
  ['brawler', 'BRAWLER'],
  ['adder', 'ADDER'],
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
    hasScreen: card.querySelector('.card-shot')?.complete && card.querySelector('.card-shot')?.naturalWidth > 0,
    hasCoinDoor: !!card.querySelector('.machine-coinbox .machine-coin-slot'),
    hasStart: !!card.querySelector('.machine-start'),
    deck: card.querySelector('.machine-controls iframe')?.getAttribute('src'),
  })));
  assert.deepEqual(cabinets, GAMES.map(([slug, title]) => ({
    slug, title, hasScreen: true, hasCoinDoor: true, hasStart: true, deck: `machine-deck.html?game=${slug}`,
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

for (const [index, [slug, title]] of GAMES.entries()) {
  test(`${title}: press its Start and reach playable controls`, async (t) => {
    const { page, origin } = await openArcade(t);
    for (let step = 0; step < index; step++) await page.keyboard.press('ArrowRight');
    await page.waitForFunction((game) => document.querySelector('.card-focused')?.dataset.gameUrl === `${game}/index.html`, slug);
    const machine = page.locator('.card-focused');
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
    // The arcade plays with the cabinet's own controls: player one's first cap, or Rally's dial.
    await page.evaluate(() => {
      window.__cabinetActionKeys = [];
      document.querySelector('canvas').addEventListener('keydown', (event) => window.__cabinetActionKeys.push(event.code));
    });
    await page.locator(slug === 'rally' ? '#paddle-dial' : '#fire-buttons .arcade-btn[data-blip="button1"]').tap({ timeout: 3000 });
    const expected = await page.evaluate(() => BLIP_GAMES[blipGameFromPath(location.pathname).slug]?.buttons?.[0]?.code || 'Space');
    await page.waitForFunction((code) => window.__cabinetActionKeys?.includes(code), expected, { timeout: 2000 });
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

test('every visit starts in the arcade, whichever front end was open last', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 900 } });
  await page.route('**/*.wasm', () => {});
  for (const last of ['basic', 'home']) {
    await page.goto(`${origin}/${last}.html`);
    await page.goto(`${origin}/index.html`);
    await page.waitForFunction(() => !document.documentElement.hasAttribute('data-cabinet-arranging'));
    assert.equal(new URL(page.url()).pathname, '/index.html', `after ${last}, BLIP opened somewhere else`);
    assert.equal(await page.evaluate(() => blipFrontend()), 'cabinet');
  }
});

// ---- Walking: the aisle's position is a decimal (data-aisle-pos); data-aisle-moving is set while it moves. ----
const aislePos = (page) => page.evaluate(() => Number(document.getElementById('game-grid').dataset.aislePos));
const aisleStill = (page) => page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'), null, { timeout: 8000 });

test('holding an arrow walks the aisle, and letting go stops on a machine', async (t) => {
  const { page } = await openArcade(t, 1280);
  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(1200);
  const passing = await aislePos(page);
  assert.ok(passing > 0.4 && passing < 2, `walked ${passing} machines in 1.2 s`);
  assert.equal(await selectedSlug(page), 'rally/index.html', 'the machine is picked on arrival, not in passing');
  await page.waitForTimeout(1800);
  await page.keyboard.up('ArrowRight');
  await aisleStill(page);
  const stop = await aislePos(page);
  assert.ok(Number.isInteger(stop) && stop >= 2 && stop <= 4, `stopped at ${stop}`);
  assert.equal(await selectedSlug(page), `${GAMES[stop][0]}/index.html`);
});

test('a drag moves the aisle with the pointer and lets go on a machine; a short one springs back', async (t) => {
  const { page } = await openArcade(t, 1280);
  await page.mouse.move(640, 300);
  await page.mouse.down();
  for (let i = 1; i <= 10; i++) { await page.mouse.move(640 - i * 20, 300); await page.waitForTimeout(30); }
  const held = await aislePos(page);
  assert.ok(held > 0.2 && held < 1, `the aisle followed to ${held}`);
  await page.waitForTimeout(300);
  await page.mouse.up();
  await aisleStill(page);
  assert.equal(await aislePos(page), 1);
  assert.equal(await selectedSlug(page), 'bouncer/index.html');
  await page.mouse.move(640, 300);
  await page.mouse.down();
  await page.mouse.move(620, 300);
  await page.mouse.move(610, 300);
  await page.mouse.up();
  await aisleStill(page);
  assert.equal(await aislePos(page), 1);
});

test('a gamepad stick walks at the pace it is pushed', async (t) => {
  const walk = async (push) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
    await page.addInitScript((x) => {
      const pad = { connected: true, axes: [0, 0], buttons: Array.from({ length: 17 }, () => ({ pressed: false, value: 0 })) };
      window.__pad = pad;
      navigator.getGamepads = () => [pad];
    }, push);
    await page.goto(`${origin}/index.html`);
    await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') && !document.documentElement.hasAttribute('data-cabinet-arranging'));
    await page.evaluate((x) => { window.__pad.axes[0] = x; }, push);
    // Its pace once up to speed, timed by the page's own clock.
    await page.waitForFunction(() => document.getElementById('game-grid').hasAttribute('data-aisle-moving'));
    await page.waitForTimeout(700);
    const pace = await page.evaluate(() => new Promise((done) => {
      const read = () => [performance.now(), Number(document.getElementById('game-grid').dataset.aislePos)];
      const [t0, p0] = read();
      setTimeout(() => { const [t1, p1] = read(); done((((p1 - p0) % 9) + 9) % 9 / ((t1 - t0) / 1000)); }, 1000);
    }));
    await page.evaluate(() => { window.__pad.axes[0] = 0; });
    await aisleStill(page);
    assert.ok(Number.isInteger(await aislePos(page)));
    return pace;
  };
  const stroll = await walk(0.4), stride = await walk(1);
  assert.ok(stride > stroll * 1.8, `full push ${stride.toFixed(2)}, light push ${stroll.toFixed(2)} machines a second`);
});

test('with reduced motion an arrow still steps exactly one whole machine', async (t) => {
  const { page } = await openArcade(t, 1280);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.evaluate(() => {
    window.__seen = [];
    new MutationObserver(() => window.__seen.push(document.getElementById('game-grid').dataset.aislePos))
      .observe(document.getElementById('game-grid'), { attributes: true, attributeFilter: ['data-aisle-pos'] });
  });
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'bouncer/index.html');
  assert.deepEqual(await page.evaluate(() => window.__seen.filter((p) => !Number.isInteger(Number(p)))), []);
  assert.equal(await page.evaluate(() => document.getElementById('game-grid').hasAttribute('data-aisle-moving')), false);
});

// ---- Sound from the machines: checked by the graph (blipArcadeSound.state()), not by ear. ----
const room = (page) => page.evaluate(() => blipArcadeSound.state());

test('the machines are silent until the first key, then the nearest is loudest and the rest pan to their sides', async (t) => {
  const { page } = await openArcade(t, 1280);
  assert.equal(await page.evaluate(() => _kioskAudioCtx === null && !(window.Howler && Howler.ctx)), true, 'no audio before any input');
  assert.equal((await room(page)).running, false);
  await page.keyboard.press('Shift');
  await page.waitForFunction(() => blipArcadeSound.state().running);
  const heard = (await room(page)).machines;
  const by = Object.fromEntries(heard.map((m) => [m.slug, m]));
  assert.deepEqual(heard.map((m) => m.slug).sort(), ['adder', 'bouncer', 'brawler', 'defender', 'rally'],
    'the faced machine and two either side');
  assert.equal(by.rally.level, 1);
  assert.equal(by.rally.pan, 0);
  assert.equal(by.bouncer.level, 0.5);
  assert.ok(by.bouncer.pan > 0 && by.adder.pan < 0, 'neighbours on their own sides');
  assert.ok(by.defender.level < by.bouncer.level);
  await page.keyboard.press('ArrowRight');
  await aisleStill(page);
  const moved = Object.fromEntries((await room(page)).machines.map((m) => [m.slug, m]));
  assert.equal(moved.bouncer.level, 1, 'walking on brings the next machine to the front');
  assert.ok(moved.rally.pan < 0);
  assert.ok(moved.bubbler, 'and the next but one into earshot');
  assert.ok(!moved.brawler, 'while the far side falls out of it');
});

test('the room is silent while muted and stops while the page is hidden', async (t) => {
  const { page } = await openArcade(t, 1280);
  await page.keyboard.press('ArrowRight');
  await page.waitForFunction(() => blipArcadeSound.state().running);
  await page.evaluate(() => blipSetMuted(true));
  assert.equal(await page.evaluate(() => blipOut(getKioskAudio()).gain.value), 0);
  await page.evaluate(() => blipSetMuted(false));
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
    document.dispatchEvent(new Event('visibilitychange'));
  });
  assert.equal((await room(page)).running, false);
  await page.evaluate(() => {
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => false });
    document.dispatchEvent(new Event('visibilitychange'));
  });
  await page.waitForFunction(() => blipArcadeSound.state().running);
});

// ---- Stepping up to play: the game page opens on the picture the arcade's zoom ended on. ----
test('a machine started from the arcade opens its game on the same screen picture, then the game', async (t) => {
  const { page, origin } = await openArcade(t, 1280);
  // Hold the game back a moment, so the picture can be seen waiting for it.
  await page.route('**/*.wasm', async (route) => { await new Promise((r) => setTimeout(r, 1500)); await route.continue(); });
  await page.locator('.card-focused .machine-coinbox').click();
  await page.locator('.card-focused .machine-start').click();
  await page.waitForURL(`${origin}/rally/index.html`);
  const shown = await page.waitForFunction(() => {
    const frame = document.getElementById('launch-frame'), canvas = document.getElementById('glcanvas');
    if (!frame) return null;
    const a = frame.getBoundingClientRect(), b = canvas.getBoundingClientRect();
    return { art: frame.style.backgroundImage, ready: document.documentElement.hasAttribute('data-game-ready'),
      over: Math.abs(a.left - b.left) < 1 && Math.abs(a.top - b.top) < 1 && Math.abs(a.width - b.width) < 1 && Math.abs(a.height - b.height) < 1 };
  });
  const seen = await shown.jsonValue();
  assert.match(seen.art, /rally\/screenshot\.png/);
  assert.equal(seen.ready, false, 'the picture waits for the game');
  assert.equal(seen.over, true, 'exactly where the game will draw');
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.waitForFunction(() => !document.getElementById('launch-frame'));
  assert.equal(await page.evaluate(() => sessionStorage.getItem('blip-cabinet-launch')), null);
});

test('a game page opened later, or another game, starts on its plain loader', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/index.html`);
  // Twenty seconds old; and fresh, but for another machine.
  for (const launch of [{ slug: 'rally', age: 20000 }, { slug: 'serpent', age: 0 }]) {
    await page.evaluate((l) => sessionStorage.setItem('blip-cabinet-launch',
      JSON.stringify({ slug: l.slug, at: Date.now() - l.age, art: `${l.slug}/screenshot.png` })), launch);
    await page.goto(`${origin}/rally/index.html`);
    await page.waitForFunction(() => typeof BlipController === 'object');
    assert.equal(await page.locator('#launch-frame').count(), 0, `${launch.slug} handover`);
  }
});

// ---- Living screens: a machine within two plays its strip; farther ones keep their still. ----
test('only the machines near you fetch and play their attract loops', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
  const fetched = [];
  page.on('request', (r) => { const m = r.url().match(/\/(\w+)\/attract\.webp$/); if (m) fetched.push(m[1]); });
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') && !document.documentElement.hasAttribute('data-cabinet-arranging'));
  await page.waitForFunction(() => document.querySelectorAll('#game-grid .card[data-loop-ready]').length === 5);
  assert.deepEqual([...fetched].sort(), ['adder', 'bouncer', 'brawler', 'defender', 'rally']);
  const strip = await page.evaluate(() => {
    const card = document.querySelector('.card[data-machine="rally"]'), img = card.querySelector('.card-loop-strip');
    return { frames: img.naturalHeight / (img.naturalWidth / (480 / 540)), playing: getComputedStyle(img).animationPlayState,
      shown: getComputedStyle(card.querySelector('.card-loop')).visibility };
  });
  assert.ok(Math.abs(strip.frames - 12) < 0.5, `${strip.frames} frames`);
  assert.equal(strip.playing, 'running');
  assert.equal(strip.shown, 'visible');
  assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector('.card[data-machine="raider"] .card-loop')).visibility), 'hidden');
  await page.keyboard.press('ArrowRight');
  await aisleStill(page);
  await page.waitForFunction(() => document.querySelector('.card[data-machine="bubbler"][data-loop-ready]'));
  assert.ok(fetched.includes('bubbler'), 'walking on fetches the next one');
  assert.ok(!fetched.includes('raider') && !fetched.includes('serpent'), `and only that: ${fetched}`);
  assert.equal(await page.evaluate(() => document.querySelector('.card[data-machine="brawler"]').hasAttribute('data-loop')), false, 'the machine left behind stops');
});

test('Escape leaves a game for the arcade, once the controls card it may have open is put away', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/meteors/index.html`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  await page.locator('#card-button').click();
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-card'));
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.documentElement.hasAttribute('data-card'));
  assert.equal(new URL(page.url()).pathname, '/meteors/index.html', 'the first Escape only closes the card');
  await page.keyboard.press('Escape');
  await page.waitForURL(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused')?.dataset.gameUrl === 'meteors/index.html');
});

test('with reduced motion Up steps up at once, and Escape steps back', async (t) => {
  const { page } = await openArcade(t, 1280);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const pose = () => page.evaluate(() => document.querySelector('.card-focused').style.transform);
  const standing = await pose();
  await page.keyboard.press('ArrowUp');
  assert.equal(await page.evaluate(() => document.getElementById('game-grid').hasAttribute('data-stepped-up')), true);
  assert.equal(await page.evaluate(() => document.getElementById('game-grid').hasAttribute('data-aisle-moving')), false);
  assert.notEqual(await pose(), standing, 'the view leans in');
  await page.keyboard.press('Escape');
  assert.equal(await page.evaluate(() => document.getElementById('game-grid').hasAttribute('data-stepped-up')), false);
  assert.equal(await pose(), standing);
});

test('the step into a machine ends on the screen of its game page, which is that machine up close', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 844 } });
  await page.route('**/*.wasm', () => {});
  await page.goto(`${origin}/bouncer/index.html`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  const screen = await page.evaluate(() => {
    const r = document.getElementById('glcanvas').getBoundingClientRect();
    return { box: [r.left, r.top, r.width, r.height], machine: document.documentElement.getAttribute('data-machine'),
      trim: getComputedStyle(document.getElementById('arcade-bezel')).borderTopColor };
  });
  assert.equal(screen.machine, 'bouncer', 'the game page wears its machine\'s colours');
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') && !document.documentElement.hasAttribute('data-cabinet-arranging'));
  const aim = await page.evaluate(() => { const b = launchScreen(); return [b.left, b.top, b.width, b.height]; });
  aim.forEach((v, i) => assert.ok(Math.abs(v - screen.box[i]) < 1, `the zoom aims at ${aim}, the screen is at ${screen.box}`));
});
