// The control deck on a cabinet: two stations (stick or pad, the game's buttons), one per player. These tests press
// each control and read back the key that reached the canvas, which is all the wasm game sees, and check that the
// page seats the stations and keeps the picture clear of them. How the deck itself is drawn and pressed (sizes,
// overlaps, the stick's gate) is tested in the blip-deck repository.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, HTTP_PORT, evaluate, waitFor, sleep, gameReady, insertCoins } from './lib/harness.mjs';

// The deck's documented parts, as '<station> <part> [n]': '2 station', '1 stick', '1 dpad', '2 buttons', '1 button 2'.
const PART = `window.deckPart = function (spec) {
  var deck = document.querySelector('blip-deck'), w = spec.split(' ');
  var station = deck && deck.shadowRoot.querySelectorAll('[part~="station"]')[w[0] - 1];
  if (!station || w[1] === 'station') return station || null;
  return station.querySelectorAll('[part~="' + w[1] + '"]')[(w[2] || 1) - 1] || null;
}; true`;

// Most of these tests read the shell's own layout and key plumbing, which do not need the wasm: unless
// `real`, it is held back so the page loads at once and the CPU stays free.
const held = new WeakSet();
async function loadGame(cdp, slug, controls, { real = false } = {}) {
  if (!real && !held.has(cdp.page)) { await cdp.page.route('**/*.wasm', () => {}); held.add(cdp.page); }
  if (real && held.has(cdp.page)) { await cdp.page.unroute('**/*.wasm'); held.delete(cdp.page); }
  await evaluate(cdp, 'true');
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${HTTP_PORT}/${slug}/index.html?controls=${controls}` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipController === 'object' && !!customElements.get('blip-deck')", 15000);
  await evaluate(cdp, PART);
  // Controls stay gated until a credit enters through the coin button.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await insertCoins(cdp.page);
    await waitFor(cdp, "!document.getElementById('need-coin-overlay').classList.contains('visible')");
  }
  if (real) await gameReady(cdp.page);
  await sleep(real ? 150 : 40);
}

/** Press a part with the mouse and report the key codes the canvas received; a cross is pressed toward `dir`. The
 * deck holds every press for at least a frame, so the release is read a moment later. */
const PRESS = (spec, dir) => `(async function () {
  var el = deckPart(${JSON.stringify(spec)});
  if (!el) return { error: 'no part ' + ${JSON.stringify(spec)} };
  var deck = document.querySelector('blip-deck'), canvas = document.getElementById('glcanvas'), seen = [];
  function note(e) { seen.push(e.type + ':' + e.code); }
  canvas.addEventListener('keydown', note);
  canvas.addEventListener('keyup', note);
  var r = el.getBoundingClientRect(), x = r.left + r.width / 2, y = r.top + r.height / 2;
  var d = ${JSON.stringify(dir || null)};
  if (d === 'left') x = r.left + r.width * 0.1;
  if (d === 'right') x = r.left + r.width * 0.9;
  if (d === 'up') y = r.top + r.height * 0.1;
  function send(type) {
    deck.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId: 7, pointerType: 'mouse', clientX: x, clientY: y }));
  }
  send('pointerdown');
  send('pointerup');
  await new Promise(function (r) { setTimeout(r, 60); });
  canvas.removeEventListener('keydown', note);
  canvas.removeEventListener('keyup', note);
  return { seen: seen };
})()`;

/** A station is on the panel (VISIBLE) and in play (LIVE: it takes input). */
const VISIBLE = (spec) => `(function () { var el = deckPart(${JSON.stringify(spec)}), r = el && el.getBoundingClientRect(); return !!r && r.width > 0 && r.height > 0; })()`;
const LIVE = (spec) => `(function () { var el = deckPart(${JSON.stringify(spec)}); return !!el && !el.hasAttribute('aria-disabled'); })()`;

const LAMPS_VISIBLE = `(function () {
  var deck = document.querySelector('blip-deck'), p = deck.getBoundingClientRect();
  return ['.deck-credit-lamp', '.deck-second-lamp'].map(function (selector) {
    var el = deck.querySelector(selector);
    if (!el) return { selector: selector, error: 'missing lamp' };
    var r = el.getBoundingClientRect();
    return { selector: selector, display: getComputedStyle(el).display, width: r.width, height: r.height,
      left: r.left, right: r.right, top: r.top, bottom: r.bottom,
      panel: { left: p.left, right: p.right, top: p.top, bottom: p.bottom }, viewport: { width: innerWidth, height: innerHeight } };
  });
})()`;

test(`brawler's deck seats two players`, async (t) => {
  const { cdp } = await openPage(t);

  await t.test('station two is always on the panel, and comes alive with a player', async () => {
    await loadGame(cdp, 'brawler', 'stick', { real: true });
    await waitFor(cdp, "document.documentElement.hasAttribute('data-open')", 10000);
    assert.equal(await evaluate(cdp, VISIBLE('2 station')), true, 'a two-player cabinet is showing only one station');
    // A cold cabinet comes up on its title screen, where station two is live: reaching for it is how a second
    // player joins.
    assert.equal(await evaluate(cdp, LIVE('2 station')), true, 'station two cannot be reached for on the title screen');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    assert.equal(await evaluate(cdp, LIVE('2 station')), true, 'station two never came alive with two players');
    assert.equal(await evaluate(cdp, VISIBLE('2 stick')), true);
    await evaluate(cdp, 'window.blipSetMode(0)');
    await sleep(120);
    assert.equal(await evaluate(cdp, LIVE('2 station')), false, 'station two stayed live after the game went back to one player');
    assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-open')"), false,
      'the title screen still advertises an open station');
  });

  await t.test('both player lamps stay visible on the deck', async () => {
    await loadGame(cdp, 'brawler', 'stick', { real: true });
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    for (const lamp of await evaluate(cdp, LAMPS_VISIBLE)) {
      assert.ok(!lamp.error, lamp.error);
      assert.equal(lamp.display, 'block', `${lamp.selector} is hidden`);
      assert.ok(lamp.width > 0 && lamp.height > 0, `${lamp.selector} has no visible size`);
      assert.ok(lamp.left >= lamp.panel.left && lamp.right <= lamp.panel.right && lamp.top >= lamp.panel.top && lamp.bottom <= lamp.panel.bottom,
        `${lamp.selector} extends beyond the deck: ${JSON.stringify(lamp)}`);
      assert.ok(lamp.left >= 0 && lamp.right <= lamp.viewport.width && lamp.top >= 0 && lamp.bottom <= lamp.viewport.height,
        `${lamp.selector} is clipped by the viewport: ${JSON.stringify(lamp)}`);
    }
  });

  // Each fighter has punch and kick buttons.
  const BUTTONS = [['1 button 1', 'KeyF', 'P1 punch'], ['1 button 2', 'KeyG', 'P1 kick'],
    ['2 button 1', 'KeyJ', 'P2 punch'], ['2 button 2', 'KeyK', 'P2 kick']];
  for (const controls of ['stick', 'pad']) {
    await t.test(`every button sends its own attack (${controls})`, async () => {
      await loadGame(cdp, 'brawler', controls);
      await evaluate(cdp, 'window.blipSetMode(1)');
      await sleep(120);
      for (const [spec, code, what] of BUTTONS) {
        const got = await evaluate(cdp, PRESS(spec));
        assert.ok(!got.error, got.error);
        assert.deepEqual(got.seen, [`keydown:${code}`, `keyup:${code}`], `${what} (${spec}) sent ${got.seen.join(' ')}`);
      }
    });
  }

  await t.test("each pad's cross walks its own fighter", async () => {
    await loadGame(cdp, 'brawler', 'pad');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    for (const [spec, dir, code] of [['1 dpad', 'left', 'KeyA'], ['1 dpad', 'right', 'KeyD'], ['1 dpad', 'up', 'KeyW'],
      ['2 dpad', 'left', 'ArrowLeft'], ['2 dpad', 'right', 'ArrowRight'], ['2 dpad', 'up', 'ArrowUp']]) {
      const got = await evaluate(cdp, PRESS(spec, dir));
      assert.ok(!got.error, got.error);
      assert.ok(got.seen.includes(`keydown:${code}`), `${spec} pressed ${dir} sent ${got.seen.join(' ') || 'nothing'}, not ${code}`);
      const strays = got.seen.filter((s) => s.startsWith('keydown:') && !s.endsWith(code));
      assert.deepEqual(strays, [], `${spec} ${dir} also sent ${strays.join(', ')}`);
    }
  });

  // The deck mirrors a keyboard player: WASD leans player one's stick, the arrows player two's.
  await t.test('a keyboard player leans their own stick, not the other one', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    const lean = (spec) => `(function () { var s = deckPart(${JSON.stringify(spec)}).style;
      return [Number(s.getPropertyValue('--dx') || 0), Number(s.getPropertyValue('--dy') || 0)]; })()`;
    const press = (key, code) => `document.dispatchEvent(new KeyboardEvent('keydown', { key: ${JSON.stringify(key)}, code: ${JSON.stringify(code)} })), true`;
    await evaluate(cdp, press('a', 'KeyA'));
    assert.deepEqual(await evaluate(cdp, lean('1 station')), [-1, 0], 'player one held left and their own stick did not lean');
    assert.deepEqual(await evaluate(cdp, lean('2 station')), [0, 0], "player one's key leaned player two's stick");
    await evaluate(cdp, press('ArrowRight', 'ArrowRight'));
    assert.deepEqual(await evaluate(cdp, lean('2 station')), [1, 0], 'player two held right and their own stick did not lean');
    assert.deepEqual(await evaluate(cdp, lean('1 station')), [-1, 0], "player two's key moved player one's stick");
  });
});

test(`a one-player game's second station is there but idle`, async (t) => {
  const { cdp } = await openPage(t);
  await t.test('serpent shows a second station, as a two-player cabinet does, that nobody can play; its one button fires', async () => {
    await loadGame(cdp, 'serpent', 'stick');
    assert.equal(await evaluate(cdp, VISIBLE('2 station')), true, 'a one-player game hides its second station');
    assert.equal(await evaluate(cdp, LIVE('2 station')), false, 'a game with no second player has a live second station');
    assert.deepEqual((await evaluate(cdp, PRESS('2 button 1'))).seen, [], "player two's button is wired on a game with no player two");
    assert.deepEqual((await evaluate(cdp, PRESS('1 button 1'))).seen, ['keydown:Space', 'keyup:Space']);
  });
});

// ---- A phone turned on its side ----------------------------------------
// Sideways the controls move into the side gutters of the picture and the picture gets the whole height.

const GEOMETRY = `(function () {
  function box(el) {
    if (!el) return null;
    var r = el.getBoundingClientRect();
    if (!r.width || !r.height) return null;
    return { x: r.left, y: r.top, w: r.width, h: r.height, r: r.right, b: r.bottom };
  }
  var c = document.getElementById('glcanvas').getBoundingClientRect();
  // The picture inside the canvas: the game letterboxes a 16:10 image into whatever box it is given.
  var ar = 640 / 400, pw = Math.min(c.width, c.height * ar), ph = pw / ar;
  var parts = ['1 stick', '1 dpad', '1 buttons', '2 stick', '2 dpad', '2 buttons'].map(function (s) { return { sel: s, b: box(deckPart(s)) }; })
    .concat(['#paddle-dial', '#paddle-dial-p2'].map(function (s) { return { sel: s, b: box(document.querySelector(s)) }; }));
  return {
    layout: document.documentElement.getAttribute('data-layout'),
    canvas: { x: c.left, y: c.top, w: c.width, h: c.height },
    picture: { x: c.left + (c.width - pw) / 2, y: c.top + (c.height - ph) / 2, w: pw, h: ph },
    controls: parts.filter(function (p) { return p.b; })
  };
})()`;

const LANDSCAPE = { width: 844, height: 390 };
const PORTRAIT = { width: 390, height: 844 };
const SIDEWAYS = [['brawler', 'pad', 1], ['brawler', 'stick', 1], ['brawler', 'pad', 2], ['brawler', 'stick', 2],
  ['rally', 'stick', 1], ['serpent', 'pad', 1]];

async function geometry(cdp, slug, controls, size, players) {
  await cdp.page.setViewportSize(size);
  await loadGame(cdp, slug, controls);
  if (players === 2) { await evaluate(cdp, 'window.blipSetMode(1)'); await sleep(200); }
  await sleep(200);
  return evaluate(cdp, GEOMETRY);
}

test(`a phone on its side puts the controls beside the picture`, async (t) => {
  const { cdp } = await openPage(t);

  await t.test('sideways, the picture gets the whole height and no control is drawn over it', async () => {
    for (const [slug, controls, players] of SIDEWAYS) {
      const g = await geometry(cdp, slug, controls, LANDSCAPE, players);
      const what = `${slug} ${controls} ${players}p`;
      assert.equal(g.layout, 'landscape', `${what}: did not switch layout`);
      assert.ok(g.canvas.y <= LANDSCAPE.height * 0.25, `${what}: the picture starts too far below the top of the screen`);
      assert.ok(g.canvas.y + g.canvas.h >= LANDSCAPE.height - 16, `${what}: the picture ends above the cabinet frame`);
      assert.ok(g.canvas.h >= 240, `${what}: the picture got only ${Math.round(g.canvas.h)}px of height`);
      assert.ok(g.controls.length > 0, `${what}: found no controls at all`);
      for (const { sel, b } of g.controls) {
        // A few pixels of overlap is the rounded corner of a button, not a control sitting on the game.
        const clear = b.r <= g.picture.x + 8 || b.x >= g.picture.x + g.picture.w - 8;
        assert.ok(clear, `${what}: ${sel} at ${Math.round(b.x)}..${Math.round(b.r)} lies over the picture `
          + `(${Math.round(g.picture.x)}..${Math.round(g.picture.x + g.picture.w)})`);
        assert.ok(b.x >= -6 && b.r <= LANDSCAPE.width + 6 && b.b <= LANDSCAPE.height + 6, `${what}: ${sel} runs off the screen`);
      }
    }
  });

  await t.test('turning the phone back puts the deck back under the picture', async () => {
    const g = await geometry(cdp, 'brawler', 'pad', PORTRAIT, 1);
    assert.equal(g.layout, 'upright');
    const top = Math.min(...g.controls.map((c) => c.b.y));
    assert.ok(g.canvas.y + g.canvas.h <= top + 2,
      `the picture runs ${Math.round(g.canvas.y + g.canvas.h)}px down, under a deck drawn from ${Math.round(top)}`);
  });
});

// ---- The deck answers the title screen ----------------------------------
// The game reports the mode as the cursor lands on it, so station two becomes active while the player is choosing.

/** Hold a key long enough for the game to sample a frame with it down. */
async function tap(cdp, key) {
  await cdp.page.keyboard.down(key);
  await sleep(140);
  await cdp.page.keyboard.up(key);
  await sleep(320);
}

test(`the deck answers the title screen straight away`, async (t) => {
  const { cdp } = await openPage(t);
  for (const controls of ['stick', 'pad']) {
    await t.test(`${controls}: the second station arrives with the cursor`, async () => {
      await loadGame(cdp, 'brawler', controls, { real: true });
      await sleep(800);
      await tap(cdp, 'KeyD');
      assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-versus')"), true,
        'the title cursor did not enter two-player mode');
      assert.equal(await evaluate(cdp, LIVE('2 station')), true, 'the second station is not live when 2 PLAYERS is selected');
      await tap(cdp, 'KeyD');
      // Still on the title, so still open and still reachable.
      assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-open')"), true);
      assert.equal(await evaluate(cdp, LIVE('2 station')), true);
    });
  }
});

test(`a second player joins by touch through their own buttons on the stick or the pad deck`, async (t) => {
  // A real tap on station two's own button against the real wasm.
  const { cdp } = await openPage(t);
  const browser = cdp.page.context().browser();

  for (const controls of ['stick', 'pad']) {
    const ctx = await browser.newContext({ viewport: { width: 900, height: 800 }, hasTouch: true });
    const page = await ctx.newPage();
    await page.goto(`http://127.0.0.1:${HTTP_PORT}/brawler/index.html?controls=${controls}`);
    await gameReady(page);
    if (await page.locator('#need-coin-overlay.visible').count()) await insertCoins(page);
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-open'), null, { timeout: 20000 });
    assert.equal(await page.evaluate(() => blipControls()), controls, `${controls}: a touchscreen keeps the deck it was asked for`);
    // Playwright's CSS reaches into the deck's shadow root: player two's first button.
    const box = await page.locator('blip-deck [part~="station"] >> nth=1').locator('[part~="button"]').first().boundingBox();
    assert.ok(box, `${controls}: player two's join button is not visible`);
    // The deck is up before the wasm reads input, so tap until it takes.
    const deadline = Date.now() + 15000;
    let open = true;
    do {
      await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForTimeout(350);
      open = await page.evaluate(() => document.documentElement.hasAttribute('data-open'));
    } while (open && Date.now() < deadline);
    assert.ok(!open, `${controls}: player two touched their own button and did not join`);
    await ctx.close();
  }
});

test(`the deck never laps over the picture`, async (t) => {
  // fillCanvas() must reserve what the deck covers, not just the bar's height.
  const { cdp } = await openPage(t);
  const GAMES = ['bouncer', 'serpent', 'brawler', 'meteors', 'rally', 'raider'];
  for (const controls of ['stick', 'pad']) {
    for (const [w, h] of [[1280, 900], [390, 844], [320, 568]]) {
      await t.test(`${controls} @ ${w}x${h}`, async () => {
        for (const game of GAMES) {
          await cdp.page.setViewportSize({ width: w, height: h });
          await loadGame(cdp, game, controls);
          await sleep(250);
          const g = await evaluate(cdp, GEOMETRY);
          const top = Math.min(h, ...g.controls.map((c) => c.b.y)); // no control on screen: the deck starts at the bottom
          assert.ok(g.canvas.h > 0, `${game}: the canvas has no height`);
          assert.ok(g.canvas.y + g.canvas.h <= top + 1,
            `${game} ${controls} @ ${w}: the picture runs to ${Math.round(g.canvas.y + g.canvas.h)} and the deck starts at ${Math.round(top)}`);
        }
      });
    }
  }
});

// A physical pad has nothing on screen of its own, so station one's lamp shows it: dim when a pad is connected, lit
// once it is used.
test(`a gamepad lights station one's lamp`, async (t) => {
  const { cdp } = await openPage(t);
  await loadGame(cdp, 'serpent', 'stick');
  const PAD = (pressed) => `(function () {
    var pad = { id: 'test pad', connected: true, axes: [0, 0],
      buttons: Array.from({ length: 16 }, function (_, i) { return { pressed: ${pressed} && i === 0, value: 0 }; }) };
    navigator.getGamepads = function () { return [pad]; };
    return true;
  })()`;
  const STATE = `document.documentElement.getAttribute('data-pad')`;
  const LAMP = `getComputedStyle(document.querySelector('.deck-credit-lamp')).getPropertyValue('--on').trim()`;
  assert.equal(await evaluate(cdp, STATE), null, 'a lamp is on with no pad connected');
  await evaluate(cdp, `document.documentElement.removeAttribute('data-credit')`);
  await evaluate(cdp, PAD(false));
  await waitFor(cdp, `${STATE} === 'idle'`, 2000);
  const idleLamp = await evaluate(cdp, LAMP);
  await evaluate(cdp, PAD(true));
  await waitFor(cdp, `${STATE} === 'active'`, 2000);
  assert.notEqual(await evaluate(cdp, LAMP), idleLamp, 'using a connected pad does not light the station lamp');
  await evaluate(cdp, PAD(false));
  await evaluate(cdp, `navigator.getGamepads = function () { return [null]; }, true`);
  await waitFor(cdp, `${STATE} === null`, 2000);
  assert.equal(await evaluate(cdp, LAMP), idleLamp, 'the lit lamp outlived the pad');
});

// Alone at a two-player cabinet the arrows are player one's (Rounds, Warlock): pressing one must not move the idle
// second station's stick as well.
test('the arrow keys move only player one\'s stick while one plays', async (t) => {
  const { cdp } = await openPage(t);
  await loadGame(cdp, 'rounds', 'stick');
  await evaluate(cdp, `(function () {
    var deck = document.querySelector('blip-deck'), press = deck.press.bind(deck);
    window.lit = {};
    deck.press = function (name, down) { window.lit[name] = !!down; return press(name, down); };
    return true;
  })()`);
  await cdp.page.keyboard.down('ArrowLeft');
  await sleep(60);
  const lit = await evaluate(cdp, 'window.lit');
  await cdp.page.keyboard.up('ArrowLeft');
  assert.equal(lit.left, true, 'player one\'s stick did not move');
  assert.notEqual(lit.p2left, true, 'player two\'s stick moved too');
});
