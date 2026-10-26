// The arcade room behind the cabinets (web/arcade-room.css, issue #65): decoration that stays out of the
// cabinets' way, keeps still unless you walk, has a lite mode, and scrolls without a jump round the loop.

import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';
import { holdHalfway, imageDifference } from './lib/arcade-room.mjs';

// Every room layer and where it shows, by #66's visibility contract: phone portrait, landscape, the rest.
// A layer a later issue adds goes here; an unlisted layer on the page fails, so does a listed one missing.
const LAYERS = {
  wall: ['shown', 'shown', 'shown'],
  trim: ['shown', 'shown', 'shown'],
  neon: ['shown', 'hidden', 'shown'],     // no wall shows in landscape (#68)
  open: ['shown', 'hidden', 'shown'],
  tiles: ['shown', 'hidden', 'shown'],
  // On its side the phone shows each machine close up, cut off below Start: its feet and the floor are below the room.
  carpet: ['shown', 'either', 'shown'],
  step: ['shown', 'either', 'shown'],
  // On its side the phone shows each machine close up, cut off below Start, so the pools are out of sight.
  pool: ['shown', 'either', 'shown'],
  shadow: ['shown', 'either', 'shown'],
  cord: ['either', 'either', 'shown'],
  // A can on Defender and a wrapper by Serpent: in view only near those machines.
  prop: ['either', 'either', 'either'],
};
const ZONES = ['trim', 'sign', 'wall', 'floor', 'edge'];
const CONFIGS = [
  { name: 'phone portrait', width: 390, height: 844, touch: true, column: 0 },
  { name: 'touch landscape', width: 844, height: 390, touch: true, column: 1 },
  { name: 'desktop', width: 1280, height: 800, touch: false, column: 2 },
  { name: 'kiosk', width: 1080, height: 1920, touch: false, column: 2, fullscreen: true },
];

async function openArcade(t, { width = 1280, height = 800, touch = false, fullscreen = false, query = '', reduce = false, capabilities = null } = {}) {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: touch, viewport: { width, height } });
  if (reduce) await page.emulateMedia({ reducedMotion: 'reduce' });
  if (fullscreen) await page.addInitScript(() => localStorage.setItem('blip-fullscreen', '1'));
  if (capabilities) await page.addInitScript((values) => {
    for (const [name, value] of Object.entries(values)) Object.defineProperty(navigator, name, { configurable: true, value });
  }, capabilities);
  await page.goto(`${origin}/index.html${query}`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
  return page;
}

const roomTravel = (page) => page.evaluate(() => Number(document.getElementById('game-grid').dataset.roomTravel));
const settled = (page) => page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'),
  null, { timeout: 15000 });

for (const config of CONFIGS) {
  test(`every room layer is decoration, shown where the contract says, on ${config.name}`, async (t) => {
    const page = await openArcade(t, config);
    const layers = await page.evaluate(() => {
      const grid = document.getElementById('game-grid').getBoundingClientRect();
      return [...document.querySelectorAll('[data-room-layer]')].map((el) => {
        const cs = getComputedStyle(el), r = el.getBoundingClientRect();
        const w = Math.min(r.right, grid.right) - Math.max(r.left, grid.left);
        const h = Math.min(r.bottom, grid.bottom) - Math.max(r.top, grid.top);
        return {
          layer: el.dataset.roomLayer, zone: el.dataset.zone, hidden: el.getAttribute('aria-hidden'),
          pointer: cs.pointerEvents, display: cs.display,
          focusable: el.querySelectorAll('a,button,input,select,textarea,iframe,[tabindex]').length + (el.tabIndex >= 0 ? 1 : 0),
          inRoom: w > 0 && h > 0 && cs.visibility !== 'hidden',
        };
      });
    });
    for (const name of Object.keys(LAYERS)) assert.ok(layers.some((l) => l.layer === name), `the ${name} layer is on the page`);
    assert.equal(await page.locator('.aisle-mural').count(), 0, 'the mosaic replaces the painted mural');
    for (const l of layers) {
      assert.ok(LAYERS[l.layer], `${l.layer} is a known room layer`);
      assert.ok(ZONES.includes(l.zone), `${l.layer} names its zone (${l.zone})`);
      assert.equal(l.hidden, 'true', `${l.layer} is aria-hidden`);
      assert.equal(l.pointer, 'none', `${l.layer} takes no pointer events`);
      assert.equal(l.focusable, 0, `${l.layer} holds nothing focusable`);
    }
    for (const [name, columns] of Object.entries(LAYERS)) {
      const mine = layers.filter((l) => l.layer === name);
      if (columns[config.column] === 'either') continue;
      if (columns[config.column] === 'shown') {
        assert.ok(mine.some((l) => l.display !== 'none' && l.inRoom), `${name} shows inside the room`);
      } else {
        assert.ok(mine.every((l) => l.display === 'none'), `${name} is hidden here`);
      }
    }
  });
}

for (const [width, height, touch] of [[390, 844, true], [1280, 800, false]]) {
  test(`a press on the screen, Start or coin door still reaches the cabinet at ${width}×${height}`, async (t) => {
    const page = await openArcade(t, { width, height, touch });
    const hits = await page.evaluate(() => {
      const card = document.querySelector('.card-focused');
      return ['.game-rom', '.machine-start', '.machine-coinbox'].map((sel) => {
        const r = card.querySelector(sel).getBoundingClientRect();
        const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return [sel, !!(hit && hit.closest('.card-focused')), hit && (hit.className || hit.tagName)];
      });
    });
    for (const [sel, inside, what] of hits) assert.ok(inside, `${sel}'s centre hits the cabinet, not ${what}`);
  });
}

// The room's own animations and transitions: on its layers, and on #game-grid's pseudo-elements.
function roomMotion() {
  const grid = document.getElementById('game-grid');
  const own = (a) => a.effect && a.effect.target &&
    (a.effect.target.closest('[data-room-layer]') || (a.effect.target === grid && a.effect.pseudoElement));
  const running = document.getAnimations().filter((a) => own(a) && a.playState === 'running').map((a) => a.animationName || a.transitionProperty || 'animation');
  const transitions = [...document.querySelectorAll('[data-room-layer]')]
    .filter((el) => getComputedStyle(el).transitionDuration.split(',').some((d) => parseFloat(d) > 0))
    .map((el) => el.dataset.roomLayer);
  return { running, transitions };
}

test('the room keeps still at rest, and under reduced motion has no warm-up or transitions', async (t) => {
  await t.test('at rest', async (t2) => {
    const page = await openArcade(t2);
    await page.waitForTimeout(2000);
    assert.deepEqual((await page.evaluate(roomMotion)).running, [], 'no room animation runs at rest');
  });
  await t.test('reduced motion', async (t2) => {
    const page = await openArcade(t2, { reduce: true });
    assert.deepEqual(await page.evaluate(roomMotion), { running: [], transitions: [] });
  });
});

// Blurred filters, blurred shadows and SVG filters on the room's own layers and rendered pseudo-elements.
function roomBlur() {
  const grid = document.getElementById('game-grid');
  const shadowBlurred = (value) => value !== 'none' && value.split(/,(?![^(]*\))/).some((shadow) => {
    const lengths = shadow.replace(/rgba?\([^)]*\)|#[0-9a-f]+|[a-z-]+\([^)]*\)/gi, '').match(/-?[\d.]+px/g) || [];
    return lengths.length >= 3 && parseFloat(lengths[2]) > 0;
  });
  const found = [];
  const look = (label, cs) => {
    if (cs.filter !== 'none') found.push(`${label} filter ${cs.filter}`);
    if (shadowBlurred(cs.boxShadow)) found.push(`${label} box-shadow`);
    if (shadowBlurred(cs.textShadow)) found.push(`${label} text-shadow`);
  };
  for (const el of document.querySelectorAll('[data-room-layer]')) {
    look(el.dataset.roomLayer, getComputedStyle(el));
    if (el.querySelector('filter,feGaussianBlur')) found.push(`${el.dataset.roomLayer} SVG filter`);
  }
  for (const pseudo of ['::before', '::after']) {
    const cs = getComputedStyle(grid, pseudo);
    if (cs.content !== 'none' && cs.display !== 'none') look(`#game-grid${pseudo}`, cs);
  }
  return found;
}

test('?room picks the room\'s look: lite drops its blur and glow, and leaves the cabinets alone', async (t) => {
  const cabinetLook = () => {
    const card = document.querySelector('.card-focused');
    const cs = getComputedStyle(card);
    return { filter: cs.filter, shadow: cs.boxShadow, background: cs.backgroundImage };
  };
  let full;
  for (const [query, mode] of [['', 'full'], ['?room=full', 'full'], ['?room=bogus', 'full'], ['?room=lite', 'lite']]) {
    await t.test(query || 'no query', async (t2) => {
      const page = await openArcade(t2, { query, capabilities: { hardwareConcurrency: 8, deviceMemory: 8 } });
      assert.equal(await page.evaluate(() => document.documentElement.dataset.room), mode);
      const look = await page.evaluate(cabinetLook);
      if (!full) full = look;
      else assert.deepEqual(look, full, 'the cabinet looks the same in either mode');
      if (mode === 'lite') {
        // With a credit banked, so the OPEN sign is lit.
        await page.keyboard.press('5');
        await page.waitForFunction(() => document.documentElement.hasAttribute('data-credit'));
        assert.deepEqual(await page.evaluate(roomBlur), [], 'nothing in the room is blurred or glows');
      }
    });
  }
});

// Each frame: how far the room has travelled, and where the wall and carpet are drawn.
function sampleRoom() {
  window.__room = [];
  const grid = document.getElementById('game-grid');
  const x = (el) => Number((/translate\(\s*(-?[\d.]+)px/.exec(el.style.transform) || [0, NaN])[1]);
  (function tick() {
    window.__room.push({ wall: x(grid.querySelector('.aisle-wall')), carpet: x(grid.querySelector('.aisle-carpet')),
      neon: x(grid.querySelector('.aisle-neon')), open: x(grid.querySelector('.aisle-open')), tiles: x(grid.querySelector('.aisle-tiles')),
      travel: Number(grid.dataset.roomTravel), pos: Number(grid.dataset.aislePos) });
    if (window.__room.length < 4000) requestAnimationFrame(tick);
  })();
}

test('the room and its signs scroll on without a jump past the last machine and the first, both ways', async (t) => {
  const page = await openArcade(t);
  // The room's layers, how fast each moves against the machines, and the machines' spacing (the next one's pose).
  const room = await page.evaluate(() => {
    const cs = getComputedStyle(document.getElementById('game-grid'));
    const next = document.querySelector('#game-grid .card[data-aisle-offset="1"]');
    return {
      spacing: Number(/calc\(-50% \+ (-?[\d.]+)px\)/.exec(next.style.transform)[1]),
      cabinets: document.querySelectorAll('#game-grid .card').length,
      // The wall stands behind the machines and scrolls at the rate its depth gives it; the floor moves with them.
      wallRate: Number(document.getElementById('game-grid').dataset.wallRate),
      layers: { wall: [Number(document.getElementById('game-grid').dataset.wallRate), parseFloat(cs.getPropertyValue('--room-wall-period'))], carpet: [1, parseFloat(cs.getPropertyValue('--room-carpet-period'))], tiles: [Number(document.getElementById('game-grid').dataset.wallRate), parseFloat(cs.getPropertyValue('--room-tiles-period'))] },
    };
  });
  await page.evaluate(sampleRoom);
  // A tap back from the first machine to the last, and one on again.
  await page.keyboard.press('ArrowLeft');
  await settled(page);
  assert.ok(Math.abs((await roomTravel(page)) + 1) < 0.01, `walked one machine back: ${await roomTravel(page)}`);
  await page.keyboard.press('ArrowRight');
  await settled(page);
  assert.ok(Math.abs(await roomTravel(page)) < 0.01, `and back to the start: ${await roomTravel(page)}`);
  // A whole circuit each way, holding the arrow.
  for (const [key, mark, dir] of [['ArrowRight', 9.4, 1], ['ArrowLeft', 0.4, -1]]) {
    await page.keyboard.down(key);
    await page.waitForFunction(([mark, dir]) => {
      const travel = Number(document.getElementById('game-grid').dataset.roomTravel);
      return dir > 0 ? travel > mark : travel < mark;
    }, [mark, dir], { timeout: 20000 });
    await page.keyboard.up(key);
    await settled(page);
  }
  const samples = await page.evaluate(() => window.__room.slice());
  assert.ok(samples.length > 60, `${samples.length} frames sampled`);
  assert.ok(samples.some((s, i) => i && Math.abs(s.pos - samples[i - 1].pos) > 4), 'the aisle wrapped round while sampled');
  // The room's travel never leaps a lap, and every layer is drawn where that travel puts it.
  for (let i = 1; i < samples.length; i++) {
    assert.ok(Math.abs(samples[i].travel - samples[i - 1].travel) < 1, `travel ${samples[i - 1].travel} → ${samples[i].travel}`);
  }
  const Aisle = (await import('../web/aisle.js')).default;
  // The signs hang on the wall: they scroll at its pace and come round once a lap of the aisle.
  room.layers.neon = room.layers.open = [room.wallRate, room.cabinets * room.spacing * room.wallRate];
  assert.ok(room.wallRate > 0.5 && room.wallRate < 1, `the wall scrolls at ${room.wallRate} of the machines' pace`);
  for (const [name, [rate, period]] of Object.entries(room.layers)) {
    let worst = 0;
    for (const s of samples) {
      let d = s[name] - Aisle.layerOffset(s.travel, room.spacing, rate, period);
      d -= period * Math.round(d / period);
      worst = Math.max(worst, Math.abs(d));
    }
    assert.ok(worst < 1, `${name} is drawn up to ${worst.toFixed(2)}px from where the walk puts it`);
  }

  // A new window size keeps the walk where it was and the room filling the window.
  const before = await roomTravel(page);
  await page.setViewportSize({ width: 900, height: 700 });
  await page.waitForTimeout(300);
  assert.equal(await roomTravel(page), before);
  const cover = await page.evaluate(() => {
    const g = document.getElementById('game-grid').getBoundingClientRect();
    return ['.aisle-wall', '.aisle-carpet', '.aisle-tiles'].map((sel) => {
      const r = document.querySelector(sel).getBoundingClientRect();
      return [sel, r.left <= g.left + 0.5 && r.right >= g.right - 0.5];
    });
  });
  for (const [sel, covers] of cover) assert.ok(covers, `${sel} still spans the room`);
});

test('the physical carpet repeats without a visible jump in either direction, full and lite', async (t) => {
  const page = await openArcade(t, { width: 390, height: 844, touch: true, query: '?room=full' });
  for (const mode of ['full', 'lite']) {
    if (mode === 'lite') { await page.goto(new URL('?room=lite', page.url()).href); await page.waitForSelector('.aisle-carpet'); }
    await page.addStyleTag({ content: '#game-grid :not(.aisle-carpet) {visibility:hidden!important}.card-pick {visibility:hidden!important}' });
    const state = await page.evaluate(() => {
      const grid = document.getElementById('game-grid'), r = grid.getBoundingClientRect();
      const next = grid.querySelector('.card[data-aisle-offset="1"]');
      return { x: r.left + r.width / 2, y: r.top + r.height * .3, unit: grid.clientWidth * .34,
        spacing: Number(/calc\(-50% \+ (-?[\d.]+)px\)/.exec(next.style.transform)[1]),
        period: parseFloat(getComputedStyle(grid).getPropertyValue('--room-carpet-period')),
        pattern: getComputedStyle(grid.querySelector('.aisle-carpet')).backgroundSize,
        clip: { x: Math.ceil(r.left + 20), y: Math.ceil(r.top + r.height * .92), width: Math.floor(r.width - 40), height: Math.floor(r.height * .07) } };
    });
    assert.ok(state.pattern.includes(`${state.period}px 160px`), 'the rendered pattern uses the movement period');
    const before = await page.screenshot({ clip: state.clip });
    await page.mouse.move(state.x, state.y); await page.mouse.down();
    await page.mouse.move(state.x - 20, state.y);
    for (const direction of [1, -1]) {
      const travel = state.period / state.spacing * direction;
      await page.mouse.move(state.x - 20 - state.unit * travel, state.y, { steps: 8 });
      await page.waitForFunction((travel) => Math.abs(Number(document.getElementById('game-grid').dataset.roomTravel) - travel) < .01, travel);
      const after = await page.screenshot({ clip: state.clip });
      const difference = await imageDifference(page, before, after);
      assert.ok(difference.fraction < .001, `${mode}, ${direction}: physical texture wrap ${JSON.stringify(difference)}`);
    }
    await page.mouse.up();
  }
});

test('the wall is warm: none of the old purple room is left', async (t) => {
  const page = await openArcade(t);
  const wall = await page.evaluate(() => getComputedStyle(document.getElementById('game-grid')).backgroundImage);
  for (const purple of ['rgba(141, 63, 176', 'rgb(18, 14, 27)', 'rgba(30, 127, 176', 'rgba(192, 48, 110']) {
    assert.ok(!wall.includes(purple), `the wall still holds ${purple}`);
  }
});

test('the full maze wall keeps square cells, seamless circuit repeats, and flat ceramic in lite mode', async (t) => {
  const page = await openArcade(t);
  const read = () => page.evaluate(() => {
    const grid = document.getElementById('game-grid'), tiles = grid.querySelector('.aisle-tiles');
    const cs = getComputedStyle(grid), r = tiles.getBoundingClientRect(), g = grid.getBoundingClientRect();
    const svg = tiles.querySelector('svg').getBoundingClientRect();
    const next = grid.querySelector('.card[data-aisle-offset="1"]');
    const spacing = Number(/calc\(-50% \+ (-?[\d.]+)px\)/.exec(next.style.transform)[1]);
    return { period: parseFloat(cs.getPropertyValue('--room-tiles-period')), width: r.width, roomWidth: g.width,
      repeats: grid.querySelectorAll('.card').length * spacing * Number(grid.dataset.wallRate) / parseFloat(cs.getPropertyValue('--room-tiles-period')),
      cellX: svg.width / 32, cellY: svg.height / 17,
      visible: Math.min(r.right, g.right) - Math.max(r.left, g.left),
      glaze: getComputedStyle(tiles.querySelector('.tile-glaze')).display,
      grout: getComputedStyle(tiles.querySelector('.tile-grout')).stroke,
      faces: [...tiles.firstElementChild.querySelectorAll(':scope > path[class]')].map((el) => getComputedStyle(el).fill) };
  });
  const full = await read();
  assert.ok(full.width > full.roomWidth + full.period, 'the mosaic includes overscan beyond both edges');
  assert.ok(Math.abs(full.repeats - Math.round(full.repeats)) < .001, 'a circuit contains whole mosaic repeats');
  assert.ok(Math.abs(full.cellX - full.cellY) < .01, 'ceramic faces remain square');
  assert.ok(full.visible >= full.roomWidth - 1, 'tiles cover the entire desktop wall');
  assert.notEqual(full.glaze, 'none');
  await page.goto(new URL('?room=lite', page.url()).href);
  await page.waitForSelector('.aisle-tiles');
  const lite = await read();
  assert.equal(lite.glaze, 'none');
  assert.deepEqual(lite.faces, full.faces, 'lite preserves every coloured face');
  assert.equal(lite.grout, full.grout, 'lite preserves the pale grout');
  await page.setViewportSize({ width: 390, height: 844 });
  await page.waitForTimeout(300);
  const phone = await read();
  assert.ok(phone.visible >= phone.roomWidth - 1, 'tiles cover the entire portrait wall');
});

test('the rendered mosaic repeats after an actual aisle lap in either direction, including after resize', async (t) => {
  const page = await openArcade(t);
  // Isolate the real room layer for an image comparison; use normal navigation and rendering throughout.
  await page.addStyleTag({ content: '#game-grid :not(.aisle-tiles):not(.aisle-tiles *) {visibility:hidden!important}' });
  const count = await page.locator('#game-grid .card').count();
  const grid = page.locator('#game-grid');
  for (const [width, height] of [[1280, 800], [390, 844]]) {
    await page.setViewportSize({ width, height });
    await page.waitForTimeout(300);
    const state = () => page.evaluate(() => ({ travel: document.getElementById('game-grid').dataset.roomTravel,
      transform: document.querySelector('.aisle-tiles').style.transform,
      box: document.querySelector('.aisle-tiles').getBoundingClientRect().toJSON() }));
    const initial = await state();
    const bounds = await grid.boundingBox();
    const clip = { x: Math.ceil(Math.max(bounds.x, initial.box.left)), y: Math.ceil(initial.box.top),
      width: Math.floor(Math.min(bounds.x + bounds.width, initial.box.right) - Math.ceil(Math.max(bounds.x, initial.box.left))),
      height: Math.floor(initial.box.height) };
    const before = await page.screenshot({ clip });
    for (const key of ['ArrowRight', 'ArrowLeft']) {
      for (let i = 0; i < count; i++) {
        await page.keyboard.press(key);
        await settled(page);
      }
      const after = await page.screenshot({ clip });
      const difference = await imageDifference(page, before, after);
      // Compositing a layer after movement can change its antialiasing by a few colour levels.
      const same = difference.fraction < .001;
      if (!same) {
        const dir = new URL('artifacts/arcade-room/', import.meta.url);
        await mkdir(dir, { recursive: true });
        await writeFile(new URL('tiles-before.png', dir), before);
        await writeFile(new URL('tiles-after.png', dir), after);
      }
      assert.ok(same, `${width}px: same ceramic image after ${key} lap: ${JSON.stringify({ difference, initial, after: await state() })}`);
    }
  }
});

const neonAnimations = (page) => page.evaluate(() => document.querySelector('.aisle-neon').getAnimations().map((a) => a.animationName));

test('the BLIP sign warms up once on arrival, then stays lit as you walk and pay', async (t) => {
  const page = await openArcade(t);
  await page.waitForTimeout(1500);
  assert.deepEqual(await neonAnimations(page), [], 'the warm-up is over');
  await page.keyboard.press('ArrowRight');
  await settled(page);
  await page.keyboard.press('5');
  await page.waitForTimeout(400);
  assert.deepEqual(await neonAnimations(page), [], 'walking and a coin do not start it again');
});

const openLit = (page) => page.evaluate(() => Number(getComputedStyle(document.querySelector('.aisle-open')).opacity));

test('automatic lite selection is resolved once before the room appears, with explicit modes taking precedence', async (t) => {
  const { browser, origin } = await openPage(t);
  const cases = [
    ['small CPU', 2, 8, '', 'lite'], ['small memory', 8, 1, '', 'lite'],
    ['missing', undefined, undefined, '', 'full'], ['invalid', NaN, -1, '', 'full'],
    ['nonfinite and zero', Infinity, 0, '', 'full'], ['strings', '2', '1', '', 'full'],
    ['positive fraction', 1.5, 8, '', 'lite'], ['memory with invalid CPU', 0, 2, '', 'lite'],
    ['explicit full', 1, 1, '?room=full', 'full'], ['explicit lite', 8, 8, '?room=lite', 'lite'],
    ['invalid query on small device', 2, 8, '?room=invalid', 'lite'],
    ['invalid query on fast device', 8, 8, '?room=invalid', 'full'],
  ];
  for (const [name, cpu, memory, query, expected] of cases) {
    const context = await browser.newContext();
    try {
      await context.addInitScript(({ cpu, memory }) => {
        Object.defineProperty(navigator, 'hardwareConcurrency', { configurable: true, value: cpu });
        Object.defineProperty(navigator, 'deviceMemory', { configurable: true, value: memory });
        window.__roomModes = [];
        new MutationObserver((records) => {
          for (const record of records) if (record.attributeName === 'data-room') window.__roomModes.push(record.target.dataset.room);
        }).observe(document, { subtree: true, attributes: true, attributeFilter: ['data-room'] });
      }, { cpu, memory });
      const page = await context.newPage();
      await page.goto(`${origin}/index.html${query}`);
      await page.waitForSelector('.aisle-neon');
      assert.deepEqual(await page.evaluate(() => ({ mode: document.documentElement.dataset.room, modes: window.__roomModes })),
        { mode: expected, modes: [expected] }, name);
    } finally { await context.close(); }
  }
});

test('portrait keeps the maze mosaic across the wall at rest and halfway walking', async (t) => {
  const page = await openArcade(t, { width: 390, height: 844, touch: true });
  for (const position of ['rest', 'half']) {
    if (position === 'half') await holdHalfway(page);
    const mosaic = await page.evaluate(() => {
      const grid = document.getElementById('game-grid').getBoundingClientRect();
      const tiles = document.querySelector('.aisle-tiles'), r = tiles.getBoundingClientRect();
      return { covers: r.left <= grid.left && r.right >= grid.right,
        characters: [...tiles.querySelectorAll('.tile-yellow,.tile-red,.tile-cyan,.tile-purple')].filter((el) => {
          const p = el.getBoundingClientRect();
          return p.right > grid.left && p.left < grid.right && p.top < grid.bottom && p.bottom > grid.top;
        }).length };
    });
    assert.ok(mosaic.covers, position + ': no bare wall is exposed');
    assert.ok(mosaic.characters >= 2, position + ': the chase motif spans the portrait wall');
  }
});

test('OPEN lights with a credit and goes dark again once a game spends it', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 800 } });
  const arcade = async () => {
    await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
      !document.documentElement.hasAttribute('data-cabinet-arranging'));
    await page.waitForTimeout(400);
  };
  await page.goto(`${origin}/index.html`);
  await arcade();
  assert.ok(await openLit(page) < 0.5, 'dim with no credit');
  await page.keyboard.press('5');
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-credit') &&
    !document.querySelector('.aisle-open').getAnimations().length);
  assert.equal(await openLit(page), 1, 'lit with a credit');
  await page.locator('.card-focused .machine-start').click();
  await page.waitForURL(`${origin}/rally/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.goto(`${origin}/index.html`);
  await arcade();
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-credit')), false);
  assert.ok(await openLit(page) < 0.5, 'dim again once the game took the credit');
});

// Where the signs must not reach: the top bar and its marquee, the walk arrows, the faced cabinet and its
// controls, and the neighbours' titles.
for (const config of [...CONFIGS, { name: 'iPhone SE', width: 320, height: 568, touch: true },
  { name: 'iPhone 14 Pro Max', width: 430, height: 932, touch: true }, { name: 'iPhone SE landscape', width: 667, height: 375, touch: true }]) {
  test(`the signs hang clear of the cabinets and controls on ${config.name}`, async (t) => {
    const page = await openArcade(t, config);
    await page.waitForTimeout(800);
    const clashes = await page.evaluate(() => {
      const box = (el) => el.getBoundingClientRect();
      const meet = (a, b) => a.left < b.right - 1 && b.left < a.right - 1 && a.top < b.bottom - 1 && b.top < a.bottom - 1;
      const keepOut = [['top bar', document.querySelector('.top-marquee-bar')], ...[...document.querySelectorAll('.pick-arrow')].map((e) => ['walk arrow', e]),
        ['faced cabinet', document.querySelector('.card-focused')],
        ...[...document.querySelectorAll('#game-grid .card[data-aisle-offset="-1"] .card-title, #game-grid .card[data-aisle-offset="1"] .card-title')].map((e) => ['neighbour title', e])];
      const found = [];
      for (const sign of document.querySelectorAll('.aisle-neon, .aisle-open')) {
        const s = box(sign);
        // Where the cabinets leave no wall (an iPhone SE's cabinet reaches the ceiling) a sign has no room at all.
        if (getComputedStyle(sign).display === 'none' || s.width < 1 || s.height < 1) continue;
        for (const [name, el] of keepOut) if (el && meet(s, box(el))) found.push(`${sign.dataset.roomLayer} meets the ${name}`);
      }
      return found;
    });
    assert.deepEqual(clashes, []);
  });
}
