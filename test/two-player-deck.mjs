// The cabinet's control deck, when the cabinet seats two players.
//
// Brawler has a 2-PLAYER badge on it, so its deck has to have two
// stations: two sticks and two sets of four caps, one per player, in
// whichever controller the player picked (the arcade stick or the game
// pad). Nothing about that is drawn by the game — it is the deck, and
// the deck is the page.
//
// What can silently break here is the wiring, not the look. Every cap
// and every stick direction funnels through one logical name in
// BlipController, and a second player means eight more of them. Get one
// wrong and two caps send the same key: player two punches and player
// one's fighter throws it. These tests press each control and read back
// the key that actually reached the canvas, which is the only thing the
// wasm game ever sees.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage, HTTP_PORT, evaluate, waitFor, sleep } from './lib/multiplayer-harness.mjs';

const ENGINE = process.env.BLIP_DECK_ENGINE || 'chromium';

async function loadGame(cdp, slug, controls) {
  await evaluate(cdp, 'true');
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${HTTP_PORT}/${slug}/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await evaluate(cdp, `blipSetControls(${JSON.stringify(controls)})`);
  await waitFor(cdp, "typeof window.BlipController === 'object'", 15000);
  // The coin wall swallows every input while it is up; insert a coin the
  // way a player would, or nothing below reaches the canvas.
  await evaluate(cdp, `(function () {
    var o = document.getElementById('need-coin-overlay');
    if (o && o.classList.contains('visible')) o.click();
    return true;
  })()`);
  await sleep(150);
}

/** Press a control and report the key codes the canvas received. `sel`
 * is a CSS selector; a d-pad arm is pressed by aiming a pointer at the
 * cross's centre offset toward that arm. */
const PRESS = (sel, dir) => `(function () {
  var el = document.querySelector(${JSON.stringify(sel)});
  if (!el) return { error: 'no element for ' + ${JSON.stringify(sel)} };
  var seen = [];
  var canvas = document.getElementById('glcanvas');
  function note(e) { seen.push(e.type + ':' + e.code); }
  canvas.addEventListener('keydown', note);
  canvas.addEventListener('keyup', note);
  var r = el.getBoundingClientRect();
  var x = r.left + r.width / 2, y = r.top + r.height / 2;
  var d = ${JSON.stringify(dir || null)};
  if (d === 'left')  x = r.left + r.width * 0.1;
  if (d === 'right') x = r.left + r.width * 0.9;
  if (d === 'up')    y = r.top + r.height * 0.1;
  if (d === 'down')  y = r.top + r.height * 0.9;
  function send(type) {
    el.dispatchEvent(new PointerEvent(type, {
      bubbles: true, cancelable: true, pointerId: 7, pointerType: 'mouse',
      clientX: x, clientY: y
    }));
  }
  send('pointerdown');
  send('pointerup');
  canvas.removeEventListener('keydown', note);
  canvas.removeEventListener('keyup', note);
  return { seen: seen, size: [r.width, r.height] };
})()`;

/** Which logical names the deck actually built, station by station. */
const DECK_NAMES = `(function () {
  function names(root) {
    if (!root) return null;
    return Array.prototype.map.call(root.querySelectorAll('[data-blip]'), function (el) {
      return el.getAttribute('data-blip');
    });
  }
  return {
    stickCaps: names(document.getElementById('fire-buttons')),
    stickCaps2: names(document.getElementById('fire-buttons-p2')),
    padAll: names(document.getElementById('snes-pad')),
    padAll2: names(document.getElementById('snes-pad-p2'))
  };
})()`;

const VISIBLE = (sel) => `(function () {
  var el = document.querySelector(${JSON.stringify(sel)});
  if (!el) return null;
  var r = el.getBoundingClientRect();
  return r.width > 0 && r.height > 0;
})()`;

test(`brawler's deck seats two players (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);

  await t.test('station two stays off the deck until the game says there is one', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    assert.equal(await evaluate(cdp, VISIBLE('#deck-p2')), false,
      'the second station is on the deck before a second player joined');

    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    assert.equal(await evaluate(cdp, VISIBLE('#deck-p2')), true,
      'the second station never appeared after the game reported two players');
    assert.equal(await evaluate(cdp, VISIBLE('#stick-base-p2')), true);
    assert.equal(await evaluate(cdp, VISIBLE('#fire-buttons-p2')), true);

    await evaluate(cdp, 'window.blipSetMode(0)');
    await sleep(120);
    assert.equal(await evaluate(cdp, VISIBLE('#deck-p2')), false,
      'the second station stayed on the deck after the game went back to one player');
  });

  await t.test('each station has four caps, and the two share no logical name', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    const deck = await evaluate(cdp, DECK_NAMES);
    assert.deepEqual(deck.stickCaps, ['button1', 'button2', 'button3', 'button4']);
    assert.deepEqual(deck.stickCaps2, ['p2button1', 'p2button2', 'p2button3', 'p2button4']);
    const shared = deck.stickCaps.filter((n) => deck.stickCaps2.includes(n));
    assert.deepEqual(shared, [], `both stations drive ${shared.join(', ')}`);
  });

  await t.test('the pad grows a second pad, with four caps and its own cross', async () => {
    await loadGame(cdp, 'brawler', 'pad');
    const deck = await evaluate(cdp, DECK_NAMES);
    assert.deepEqual(deck.padAll,
      ['up', 'right', 'down', 'left', 'button1', 'button2', 'button3', 'button4']);
    assert.deepEqual(deck.padAll2,
      ['p2up', 'p2right', 'p2down', 'p2left', 'p2button1', 'p2button2', 'p2button3', 'p2button4']);
  });

  // The fighter reads four attack keys per player (crates/brawler/src/
  // main.rs: P1_SHARING / P2). A cap that sends the wrong one of them is
  // the difference between a high kick and a low punch.
  const CAPS = [
    ['#fire-buttons .arcade-btn:nth-child(1)', 'KeyF', 'P1 low punch'],
    ['#fire-buttons .arcade-btn:nth-child(2)', 'KeyR', 'P1 high punch'],
    ['#fire-buttons .arcade-btn:nth-child(3)', 'KeyG', 'P1 low kick'],
    ['#fire-buttons .arcade-btn:nth-child(4)', 'KeyT', 'P1 high kick'],
    ['#fire-buttons-p2 .arcade-btn:nth-child(1)', 'KeyJ', 'P2 low punch'],
    ['#fire-buttons-p2 .arcade-btn:nth-child(2)', 'KeyU', 'P2 high punch'],
    ['#fire-buttons-p2 .arcade-btn:nth-child(3)', 'KeyK', 'P2 low kick'],
    ['#fire-buttons-p2 .arcade-btn:nth-child(4)', 'KeyI', 'P2 high kick']
  ];

  await t.test('every cap on the stick deck sends its own attack', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    for (const [sel, code, what] of CAPS) {
      const got = await evaluate(cdp, PRESS(sel));
      assert.ok(!got.error, got.error);
      assert.deepEqual(got.seen, [`keydown:${code}`, `keyup:${code}`],
        `${what} (${sel}) sent ${got.seen.join(' ')}`);
    }
  });

  await t.test('every cap on the pads sends its own attack', async () => {
    await loadGame(cdp, 'brawler', 'pad');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    const padCaps = CAPS.map(([sel, code, what], i) =>
      [`${i < 4 ? '#snes-pad' : '#snes-pad-p2'} .snes-face .snes-btn:nth-child(${(i % 4) + 1})`, code, what]);
    for (const [sel, code, what] of padCaps) {
      const got = await evaluate(cdp, PRESS(sel));
      assert.ok(!got.error, got.error);
      assert.deepEqual(got.seen, [`keydown:${code}`, `keyup:${code}`],
        `${what} (${sel}) sent ${got.seen.join(' ')}`);
    }
  });

  await t.test("each pad's cross walks its own fighter", async () => {
    await loadGame(cdp, 'brawler', 'pad');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    const walks = [
      ['#snes-pad .snes-dpad', 'left', 'KeyA'],
      ['#snes-pad .snes-dpad', 'right', 'KeyD'],
      ['#snes-pad .snes-dpad', 'up', 'KeyW'],
      ['#snes-pad-p2 .snes-dpad', 'left', 'ArrowLeft'],
      ['#snes-pad-p2 .snes-dpad', 'right', 'ArrowRight'],
      ['#snes-pad-p2 .snes-dpad', 'up', 'ArrowUp']
    ];
    for (const [sel, dir, code] of walks) {
      const got = await evaluate(cdp, PRESS(sel, dir));
      assert.ok(!got.error, got.error);
      assert.ok(got.seen.includes(`keydown:${code}`),
        `${sel} pressed ${dir} sent ${got.seen.join(' ') || 'nothing'}, not ${code}`);
      // A cross that reports a direction must never report the other
      // player's: that is two fighters answering one thumb.
      const strays = got.seen.filter((s) => s.startsWith('keydown:') && !s.endsWith(code));
      assert.deepEqual(strays, [], `${sel} ${dir} also sent ${strays.join(', ')}`);
    }
  });

  // The deck doubles as a read-out for a keyboard player: whichever
  // player's key is pressed, that player's stick leans. WASD is player
  // one's and the arrows are player two's, so a deck that still reads
  // the arrows as "up" leans the wrong stick.
  await t.test('a keyboard player leans their own stick, not the other one', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    // Neutral is 0, and an untouched stick has not been written at all —
    // both mean "not leaning", so read them as numbers.
    const lean = (sel) => `(function () {
      var el = document.querySelector(${JSON.stringify('%s')});
      return [Number(el.style.getPropertyValue('--dx') || 0),
              Number(el.style.getPropertyValue('--dy') || 0)];
    })()`.replace('%s', sel);
    const press = (key, code) => `(function () {
      document.dispatchEvent(new KeyboardEvent('keydown', { key: ${JSON.stringify(key)}, code: ${JSON.stringify(code)} }));
      return true;
    })()`;

    await evaluate(cdp, press('a', 'KeyA'));
    assert.deepEqual(await evaluate(cdp, lean('#stick-base')), [-1, 0],
      'player one held left and their own stick did not lean');
    assert.deepEqual(await evaluate(cdp, lean('#stick-base-p2')), [0, 0],
      "player one's key leaned player two's stick");

    await evaluate(cdp, press('ArrowRight', 'ArrowRight'));
    assert.deepEqual(await evaluate(cdp, lean('#stick-base-p2')), [1, 0],
      'player two held right and their own stick did not lean');
    assert.deepEqual(await evaluate(cdp, lean('#stick-base')), [-1, 0],
      "player two's key moved player one's stick");
  });
});

test(`a one-player cabinet still has one station (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);

  await t.test('serpent builds no second station and no second pad', async () => {
    await loadGame(cdp, 'serpent', 'stick');
    assert.equal(await evaluate(cdp, VISIBLE('#deck-p2')), false);
    assert.equal(await evaluate(cdp, "document.documentElement.hasAttribute('data-players')"), false);
    const deck = await evaluate(cdp, DECK_NAMES);
    assert.deepEqual(deck.stickCaps, ['button1']);
    // The markup is there on every page — it just must never be wired
    // to anything on a cabinet that seats one.
    assert.deepEqual(deck.stickCaps2, []);
  });

  await t.test("serpent's one cap still fires", async () => {
    await loadGame(cdp, 'serpent', 'stick');
    const got = await evaluate(cdp, PRESS('#fire-buttons .arcade-btn:nth-child(1)'));
    assert.deepEqual(got.seen, ['keydown:Space', 'keyup:Space']);
  });
});
