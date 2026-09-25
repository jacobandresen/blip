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
import { openPage, HTTP_PORT, evaluate, waitFor, sleep } from './lib/harness.mjs';

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

/** Whether a station is in play, as opposed to merely on the panel.
 * A two-player cabinet shows both stations in a one-player game too —
 * that is what the empty half of an arcade panel looks like — so the
 * question is never "is it there" but "is anybody at it". */
const LIVE = (sel) => `(function () {
  var el = document.querySelector(${JSON.stringify(sel)});
  if (!el) return null;
  var r = el.getBoundingClientRect();
  if (!(r.width > 0 && r.height > 0)) return false;
  // Opacity, not pointer-events: a pad sets pointer-events none on
  // itself by design and lets only its buttons take input.
  return parseFloat(getComputedStyle(el).opacity) > 0.9;
})()`;

const VISIBLE = (sel) => `(function () {
  var el = document.querySelector(${JSON.stringify(sel)});
  if (!el) return null;
  var r = el.getBoundingClientRect();
  return r.width > 0 && r.height > 0;
})()`;

test(`brawler's deck seats two players (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);

  await t.test('station two is always on the panel, and comes alive with a player', async () => {
    await loadGame(cdp, 'brawler', 'stick');
    // There from the start: the machine takes two, and a player should
    // be able to see that before choosing anything.
    assert.equal(await evaluate(cdp, VISIBLE('#deck-p2')), true,
      'a two-player cabinet is showing only one station');
    assert.equal(await evaluate(cdp, LIVE('#deck-p2')), false,
      'station two is live before a second player joined');

    await evaluate(cdp, 'window.blipSetMode(1)');
    await sleep(120);
    assert.equal(await evaluate(cdp, LIVE('#deck-p2')), true,
      'station two never came alive after the game reported two players');
    assert.equal(await evaluate(cdp, VISIBLE('#stick-base-p2')), true);
    assert.equal(await evaluate(cdp, VISIBLE('#fire-buttons-p2')), true);

    await evaluate(cdp, 'window.blipSetMode(0)');
    await sleep(120);
    assert.equal(await evaluate(cdp, LIVE('#deck-p2')), false,
      'station two stayed live after the game went back to one player');
    // And it says who is at it, rather than leaving an idle stick
    // claiming to be a second player.
    assert.equal(await evaluate(cdp,
      "document.querySelector('#deck-p2 .deck-tag').textContent"), 'CPU');
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

// ---- A phone turned on its side ----------------------------------------
//
// Upright the deck is a strip across the bottom and the picture sits
// above it. Turned sideways that strip is a third of the screen's
// height and the picture becomes a slot — while a 5:4 picture in a 2:1
// screen leaves a wide black gutter down each side doing nothing. So
// sideways the controls move into the gutters, and the picture gets
// the whole height. These tests measure that it actually does, and
// that nothing lands on top of the picture.

const GEOMETRY = `(function () {
  function box(sel) {
    var el = document.querySelector(sel);
    if (!el) return null;
    var r = el.getBoundingClientRect();
    if (!r.width || !r.height) return null;
    return { x: r.left, y: r.top, w: r.width, h: r.height, r: r.right, b: r.bottom };
  }
  var deck = document.getElementById('topbar').getBoundingClientRect();
  var c = document.getElementById('glcanvas').getBoundingClientRect();
  // The picture inside the canvas: the game letterboxes a 5:4 image
  // into whatever box it is given, so the picture is the 5:4 rectangle
  // centred in it — which is what the controls have to keep off.
  var ar = 640 / 400;
  var pw = Math.min(c.width, c.height * ar), ph = pw / ar;
  return {
    layout: document.documentElement.getAttribute('data-layout'),
    vp: [innerWidth, innerHeight],
    deck: { x: deck.left, w: deck.width, r: deck.right },
    canvas: { x: c.left, y: c.top, w: c.width, h: c.height },
    picture: { x: c.left + (c.width - pw) / 2, y: c.top + (c.height - ph) / 2, w: pw, h: ph },
    controls: ['#stick-base', '#fire-buttons', '#stick-base-p2', '#fire-buttons-p2',
               '#snes-pad .snes-dpad', '#snes-pad .snes-face',
               '#snes-pad-p2 .snes-dpad', '#snes-pad-p2 .snes-face',
               '#paddle-dial', '#paddle-dial-p2']
      .map(function (s) { var b = box(s); return b && { sel: s, b: b }; })
      .filter(Boolean)
  };
})()`;

const LANDSCAPE = { width: 844, height: 390 };
const PORTRAIT = { width: 390, height: 844 };

async function geometry(cdp, slug, controls, size, players) {
  await cdp.page.setViewportSize(size);
  await loadGame(cdp, slug, controls);
  if (players === 2) { await evaluate(cdp, 'window.blipSetMode(1)'); await sleep(200); }
  await sleep(200);
  return evaluate(cdp, GEOMETRY);
}

test(`a phone on its side puts the controls beside the picture (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);

  await t.test('sideways, the picture gets the whole height of the screen', async () => {
    for (const [slug, controls, players] of
         [['brawler', 'pad', 1], ['brawler', 'stick', 1],
          ['brawler', 'pad', 2], ['brawler', 'stick', 2],
          ['rally', 'stick', 1], ['serpent', 'pad', 1]]) {
      const g = await geometry(cdp, slug, controls, LANDSCAPE, players);
      const what = `${slug} ${controls} ${players}p`;
      assert.equal(g.layout, 'landscape', `${what}: did not switch layout`);
      // The deck used to take 78-116px of a 390px screen. Whatever is
      // left for the canvas now, it must be most of what the marquee
      // does not already have.
      assert.ok(g.canvas.h >= LANDSCAPE.height - 70,
        `${what}: the picture got ${Math.round(g.canvas.h)}px of ${LANDSCAPE.height}`);
    }
  });

  await t.test('and no control is drawn over the picture', async () => {
    for (const [slug, controls, players] of
         [['brawler', 'pad', 1], ['brawler', 'stick', 1],
          ['brawler', 'pad', 2], ['brawler', 'stick', 2],
          ['rally', 'stick', 1], ['serpent', 'pad', 1]]) {
      const g = await geometry(cdp, slug, controls, LANDSCAPE, players);
      const what = `${slug} ${controls} ${players}p`;
      assert.ok(g.controls.length > 0, `${what}: found no controls at all`);
      for (const { sel, b } of g.controls) {
        // Every control is in a gutter: entirely left of the picture or
        // entirely right of it. A few pixels of overlap is the rounded
        // corner of a cap, not a control sitting on the fight.
        const clear = b.r <= g.picture.x + 8 || b.x >= g.picture.x + g.picture.w - 8;
        assert.ok(clear, `${what}: ${sel} at ${Math.round(b.x)}..${Math.round(b.r)} `
          + `lies over the picture (${Math.round(g.picture.x)}..`
          + `${Math.round(g.picture.x + g.picture.w)})`);
        // And on the screen.
        assert.ok(b.x >= -6 && b.r <= LANDSCAPE.width + 6,
          `${what}: ${sel} runs off the side of the screen`);
        assert.ok(b.b <= LANDSCAPE.height + 6,
          `${what}: ${sel} runs off the bottom of the screen`);
      }
    }
  });

  await t.test("each player's controls are together, and the players are apart", async () => {
    // Two rules, and the second is the one that keeps getting broken.
    // Within a station the stick is on the left and the buttons on the
    // right, whichever controller is on — the deck must not be two
    // different controllers depending on which you picked. Between the
    // stations there is a gap, and it has to be bigger than any gap
    // inside one: with each station spread across its own half of the
    // panel, player one's buttons ended up against player two's stick
    // and the daylight was in the wrong place.
    for (const size of [PORTRAIT, LANDSCAPE]) {
      for (const [controls, steer, hit, steer2, hit2] of
           [['stick', '#stick-base', '#fire-buttons', '#stick-base-p2', '#fire-buttons-p2'],
            ['pad', '#snes-pad .snes-dpad', '#snes-pad .snes-face',
             '#snes-pad-p2 .snes-dpad', '#snes-pad-p2 .snes-face']]) {
        const g = await geometry(cdp, 'brawler', controls, size, 2);
        const at = (sel) => {
          const c = g.controls.find((x) => x.sel === sel);
          assert.ok(c, `${controls} @ ${size.width}: ${sel} is missing`);
          return c.b;
        };
        const where = `${controls} @ ${size.width}`;
        const [a, b, c, d] = [at(steer), at(hit), at(steer2), at(hit2)];
        // Stick left of buttons, on both stations. Compared by centre,
        // because a stick's hit-box is wider than the stick drawn in it
        // and the two boxes are allowed to touch.
        const mid = (r) => r.x + r.w / 2;
        assert.ok(mid(a) < mid(b), `${where}: player one's stick is not left of their buttons`);
        assert.ok(mid(c) < mid(d), `${where}: player two's stick is not left of their buttons`);
        // Player one entirely left of player two.
        assert.ok(b.r <= c.x, `${where}: the two stations overlap`);
        // And the gap between them beats the gap inside either.
        const between = c.x - b.r;
        const inside = Math.max(b.x - a.r, d.x - c.r);
        assert.ok(between > inside * 1.4 && between > 24,
          `${where}: ${Math.round(between)}px between the players against `
          + `${Math.round(inside)}px inside a station — the daylight is in the wrong place`);
      }
    }
  });

  await t.test('a one-player deck still spreads its one station', async () => {
    // A cabinet with one station has nothing to be apart FROM, so its
    // stick and buttons go to the two ends of the panel instead — the
    // layout the game pad has always had.
    for (const [controls, steer, hit] of
         [['stick', '#stick-base', '#fire-buttons'],
          ['pad', '#snes-pad .snes-dpad', '#snes-pad .snes-face']]) {
      const g = await geometry(cdp, 'serpent', controls, PORTRAIT, 1);
      const by = (sel) => g.controls.find((x) => x.sel === sel);
      const a = by(steer), b = by(hit);
      if (!a || !b) continue;   // serpent draws one cap; the pad has both
      assert.ok(a.b.r <= b.b.x, `${controls}: the stick is not left of the buttons`);
      const edge = g.deck.w * 0.3;
      assert.ok(a.b.x - g.deck.x < edge && g.deck.r - b.b.r < edge,
        `${controls}: the one station is not spread across the panel`);
    }
  });

  await t.test('both players get the same deck, not mirror images', async () => {
    for (const controls of ['stick', 'pad']) {
      const g = await geometry(cdp, 'brawler', controls, LANDSCAPE, 2);
      const by = (s) => g.controls.find((c) => c.sel === s);
      const pairs = controls === 'stick'
        ? [['#stick-base', '#fire-buttons'], ['#stick-base-p2', '#fire-buttons-p2']]
        : [['#snes-pad .snes-dpad', '#snes-pad .snes-face'],
           ['#snes-pad-p2 .snes-dpad', '#snes-pad-p2 .snes-face']];
      for (const [steer, hit] of pairs) {
        const a = by(steer), b = by(hit);
        assert.ok(a && b, `${controls}: ${steer} or ${hit} is missing`);
        assert.ok(a.b.r <= b.b.x,
          `${controls}: ${steer} is not left of ${hit} — station is mirrored`);
      }
    }
  });

  await t.test('turning the phone back puts the deck back under the picture', async () => {
    const g = await geometry(cdp, 'brawler', 'pad', PORTRAIT, 1);
    assert.equal(g.layout, 'upright');
    const bar = await evaluate(cdp,
      "Math.round(document.getElementById('topbar').getBoundingClientRect().height)");
    assert.ok(bar > 60, `the deck collapsed in portrait: ${bar}px`);
    // The canvas stops above the deck rather than running under it.
    assert.ok(g.canvas.y + g.canvas.h <= PORTRAIT.height - bar + 2,
      `the picture runs ${Math.round(g.canvas.y + g.canvas.h)}px down, under a deck at `
      + `${PORTRAIT.height - bar}`);
  });
});

// ---- the deck answers the title screen ----------------------------------
//
// Choosing between one player and two is a question, and the answer a
// player is looking for is whether a second stick appears in front of
// them. Told only once the choice is confirmed, the deck is reporting
// history; told on the select screen, it has answered a question
// nobody is asking any more. So the game reports the mode as the
// cursor lands on it.
//
// This drives the real wasm — a coin in the slot and a key on the
// keyboard — because the thing being tested is whether the game calls
// out at the right moment, and calling window.blipSetMode by hand would
// test only that the page still listens.

async function bootGame(cdp, controls) {
  await loadGame(cdp, 'brawler', controls);
  await waitFor(cdp, "document.getElementById('loader').style.display === 'none'", 20000);
  // The coin wall goes up on a cold cabinet; the title screen does not
  // read a key until it comes down.
  await evaluate(cdp, `(function () {
    var o = document.getElementById('need-coin-overlay');
    if (o && o.classList.contains('visible')) o.click();
    return true;
  })()`);
  await sleep(800);
}

/** Hold a key long enough for the game to sample a frame with it down. */
async function tap(cdp, key) {
  await cdp.page.keyboard.down(key);
  await sleep(140);
  await cdp.page.keyboard.up(key);
  await sleep(320);
}

const PLAYERS = "document.documentElement.getAttribute('data-players')";

test(`the deck answers the title screen straight away (${ENGINE})`, async (t) => {
  const { cdp } = await openPage(t, ENGINE);

  for (const controls of ['stick', 'pad']) {
    // Which element *is* station two depends on which controller is
    // on: the joystick deck hides its pads and the pad deck hides its
    // sticks, so asking after the wrong one asks after something that
    // is deliberately not there.
    const station2 = controls === 'stick' ? '#deck-p2' : '#snes-pad-p2';
    await t.test(`${controls}: the second station arrives with the cursor`, async () => {
      await bootGame(cdp, controls);
      assert.equal(await evaluate(cdp, PLAYERS), '1',
        'the cabinet started with two stations on the deck');

      // The title menu is two entries; either direction moves between
      // them, and both of them have to be answered.
      await tap(cdp, 'KeyD');
      assert.equal(await evaluate(cdp, PLAYERS), '2',
        'the cursor reached 2 PLAYERS and no second station appeared');
      assert.equal(await evaluate(cdp, LIVE(station2)), true,
        `the second station is reported but ${station2} is not in play`);

      await tap(cdp, 'KeyD');
      assert.equal(await evaluate(cdp, PLAYERS), '1',
        'the cursor went back to 1 PLAYER and the second station stayed');
      assert.equal(await evaluate(cdp, LIVE(station2)), false);

      // And the choice survives being confirmed: the select screen and
      // the match that follows keep whatever the title said.
      await tap(cdp, 'KeyA');
      assert.equal(await evaluate(cdp, PLAYERS), '2');
      await tap(cdp, 'KeyF');           // player one's low punch: confirm
      assert.equal(await evaluate(cdp, PLAYERS), '2',
        'confirming 2 PLAYERS took the second station away again');
    });
  }
});
