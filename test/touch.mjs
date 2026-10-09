// Touch input in the games: what a finger on the touch strip or the deck sends
// to the canvas.
//
//   node --test --test-concurrency=1 test/touch.mjs        (after ./build_web.sh)

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

/** A game page on a 390x844 phone with five coins in and the canvas's keys recorded. */
async function phone(t, slug) {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  await page.goto(`${origin}/${slug}/index.html`);
  await page.waitForFunction(() =>
    typeof BlipController === 'object' && document.getElementById('loader').style.display === 'none', null, { timeout: 30000 });
  await page.evaluate(() => {
    for (let i = 0; i < 5; i++) document.getElementById('insert-coin-btn').click();
    window.__keys = [];
    const canvas = document.getElementById('glcanvas');
    for (const type of ['keydown', 'keyup']) {
      canvas.addEventListener(type, (e) => window.__keys.push((type === 'keydown' ? '+' : '-') + e.code));
    }
  });
  // The title screen can re-seat the strip (Bubbler opens its second seat): wait for it to hold still.
  let last = '';
  for (let steady = 0; steady < 3;) {
    await page.waitForTimeout(250);
    const now = await page.evaluate(() => JSON.stringify(document.querySelector('.ts-glass')?.getBoundingClientRect()));
    steady = now === last ? steady + 1 : 0;
    last = now;
  }

  const fingers = new Map();
  const send = (type, points) => cdp.send('Input.dispatchTouchEvent', { type, touchPoints: points });
  return {
    page,
    async down(id, x, y) { fingers.set(id, { id, x, y }); await send('touchStart', [...fingers.values()]); },
    async move(id, x, y) { fingers.set(id, { id, x, y }); await send('touchMove', [...fingers.values()]); },
    async up(id) {
      const finger = fingers.get(id);
      fingers.delete(id);
      await send('touchEnd', [finger]);
    },
    /** The keys the canvas heard since the last call, in order, as "+Code" and "-Code". */
    async keys() {
      await page.waitForTimeout(120);
      return (await page.evaluate(() => window.__keys.splice(0))).join(' ');
    },
    strip: () => page.evaluate(() => {
      const r = document.querySelector('.ts-glass').getBoundingClientRect();
      return { x: r.left, y: r.top, w: r.width, h: r.height };
    }),
  };
}

test('Serpent: a swipe on the strip steers, a longer drag chains turns, a tap starts', async (t) => {
  const p = await phone(t, 'serpent');
  const { x, y, w, h } = await p.strip();
  const [cx, cy] = [x + w / 2, y + h / 2];

  await p.down(1, cx, cy); await p.up(1);
  assert.equal(await p.keys(), '+Space -Space', 'a tap is the fire key');

  await p.down(1, cx - 40, cy); await p.move(1, cx, cy); await p.move(1, cx + 40, cy); await p.up(1);
  assert.equal(await p.keys(), '+ArrowRight -ArrowRight', 'a straight drag turns once');

  await p.down(1, cx, cy + 30); await p.move(1, cx, cy + 10); await p.move(1, cx, cy - 20); await p.up(1);
  assert.equal(await p.keys(), '+ArrowUp -ArrowUp');

  await p.down(1, cx - 30, cy - 20); await p.move(1, cx, cy - 20); await p.move(1, cx, cy + 10); await p.up(1);
  assert.equal(await p.keys(), '+ArrowRight -ArrowRight +ArrowDown -ArrowDown', 'one drag can turn a corner');

  await p.down(1, cx, cy); await p.move(1, cx + 30, cy + 20); await p.up(1);
  assert.equal(await p.keys(), '+ArrowRight -ArrowRight', 'a diagonal turns along its longer side');
});

for (const slug of ['bouncer', 'galactic_defender']) {
  test(`${slug}: the strip spans the whole width of the picture`, async (t) => {
    const p = await phone(t, slug);
    const { x, y, w, h } = await p.strip();
    const reads = [];
    for (const at of [x + 2, x + w / 2, x + w - 2]) {
      await p.down(1, at, y + h / 2);
      reads.push(Number((await p.page.evaluate(() => blipTouchPos(0, 0))).toFixed(1)));
      await p.up(1);
    }
    assert.deepEqual(reads, [0, 0.5, 1], 'left edge, middle and right edge of the strip');
  });
}

test('Rally: a finger on the strip drags the bat by distance, at the picture\'s scale', async (t) => {
  const p = await phone(t, 'rally');
  const { x, y, w, h } = await p.strip();
  const canvasHeight = await p.page.evaluate(() => document.getElementById('glcanvas').getBoundingClientRect().height);
  const read = async () => { await p.page.waitForTimeout(150); return p.page.evaluate(() => blipTouchPos(0, 1)); };
  const left = x + w * 0.25;                     // player one's half; the middle is the line between the players
  await p.down(1, left, y + 10);
  const before = await read();
  await p.move(1, left, y + 10 + 50);
  const after = await read();
  assert.ok(Math.abs((after - before) * canvasHeight - 50) < 1, 'fifty pixels of finger are fifty pixels of picture');
});

test('Bubbler: touch blows a bubble, a slide runs, a flick up jumps, a lift stops', async (t) => {
  const p = await phone(t, 'bubbler');
  const { x, y, w, h } = await p.strip();
  const [left, cy] = [x + w * 0.25, y + h / 2];   // the left half is player one's

  await p.down(1, left, cy);
  const bubble = await p.keys();
  await p.move(1, left + 8, cy);
  const small = await p.keys();
  await p.move(1, left + 14, cy);
  const run = await p.keys();
  await p.move(1, left + 14, cy - 25);
  const jump = await p.keys();
  await p.up(1);
  const stop = await p.keys();

  assert.equal(bubble, '+KeyF -KeyF', 'touching blows one bubble');
  assert.equal(small, '', 'a slide under 10px does nothing');
  assert.equal(run, '+KeyD', 'a slide past 10px runs right');
  assert.equal(jump, '+KeyG -KeyG', 'a flick up jumps');
  assert.equal(stop, '-KeyD', 'lifting the finger stops the run');
});

test('Meteors: one finger holds the d-pad while another taps the buttons', async (t) => {
  const p = await phone(t, 'meteors');
  const pad = await p.page.evaluate(() => {
    const centre = (selector) => [...document.querySelectorAll(selector)]
      .filter((el) => el.getBoundingClientRect().width > 0).map((el) => {
        const r = el.getBoundingClientRect();
        return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width };
      });
    return { dpad: centre('.snes-dpad')[0], buttons: centre('.snes-btn') };
  });
  const dpad = pad.dpad;
  await p.down(1, dpad.x + dpad.w * 0.35, dpad.y);
  assert.equal(await p.keys(), '+ArrowRight');

  await p.down(2, pad.buttons[0].x, pad.buttons[0].y);
  assert.equal(await p.keys(), '+Space', 'the direction stays held while a button goes down');
  await p.up(2);
  assert.equal(await p.keys(), '-Space', 'lifting the button leaves the direction held');

  await p.move(1, dpad.x - dpad.w * 0.35, dpad.y);
  assert.equal(await p.keys(), '+ArrowLeft -ArrowRight', 'sliding across the d-pad changes direction');
  await p.up(1);
  assert.equal(await p.keys(), '-ArrowLeft');
});

test('Rally: in two-player play the strip is split down its middle, and each half is one player\'s', async (t) => {
  const p = await phone(t, 'rally');
  const { x, y, w, h } = await p.strip();
  await p.down(1, x + w * 0.9, y + h / 2);        // on the title, the right half starts a two-player game
  await p.up(1);
  await p.page.waitForFunction(() => document.documentElement.hasAttribute('data-versus'));
  const owner = async (fraction) => {
    await p.down(1, x + w * fraction, y + h / 2);
    const slots = await p.page.evaluate(() => [blipTouchDown(0), blipTouchDown(1)]);
    await p.up(1);
    return slots[0] ? 'P1' : slots[1] ? 'P2' : 'nobody';
  };
  assert.deepEqual([await owner(0.1), await owner(0.45), await owner(0.55), await owner(0.9)], ['P1', 'P1', 'P2', 'P2']);
});

test('a long press raises no context menu, except in a text field', async (t) => {
  const p = await phone(t, 'serpent');
  const prevented = (selector) => p.page.evaluate((sel) => {
    const event = new MouseEvent('contextmenu', { bubbles: true, cancelable: true });
    document.querySelector(sel).dispatchEvent(event);
    return event.defaultPrevented;
  }, selector);
  assert.equal(await prevented('#glcanvas'), true);
  assert.equal(await prevented('#topbar'), true);
  await p.page.evaluate(() => {
    const field = document.createElement('input');
    field.id = 'name-field';
    document.body.append(field);
  });
  assert.equal(await prevented('#name-field'), false, 'a name can still be copied and pasted');
});

for (const held of [true, false]) {
  test(`fullscreen ${held ? 'stays on the game while a finger is held down' : 'returns to the cabinet after two idle minutes'}`, async (t) => {
    const p = await phone(t, 'galactic_defender');
    await p.page.evaluate(() => blipSetFullscreen(true));
    await p.page.clock.install();
    const { x, y, w, h } = await p.strip();
    if (held) await p.down(1, x + w / 2, y + h / 2);
    await p.page.clock.fastForward(180000);
    if (held) {
      await new Promise((resolve) => setTimeout(resolve, 1500));
      assert.match(p.page.url(), /galactic_defender/);
    } else {
      await p.page.waitForURL((url) => !url.pathname.includes('galactic_defender'), { timeout: 10000 });
    }
  });
}

for (const slug of ['serpent', 'bouncer', 'rally', 'galactic_defender', 'meteors', 'sky_raider', 'brawler', 'bubbler', 'adder']) {
  test(`${slug} keeps running when the clock jumps back and then a long way forward`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
    await page.goto(`${origin}/${slug}/index.html`);
    await page.waitForFunction(() => document.getElementById('loader').style.display === 'none', null, { timeout: 30000 });
    await page.clock.install();
    await page.clock.fastForward(180000);
    await new Promise((resolve) => setTimeout(resolve, 3000));
    assert.equal(await page.evaluate(() => document.getElementById('loader').style.display), 'none', 'the game stopped');
  });
}

for (const slug of ['bouncer', 'galactic_defender']) {
  test(`${slug}: a second finger does not take the ship from the first, or drop it on lifting`, async (t) => {
    const p = await phone(t, slug);
    const { x, y, w, h } = await p.strip();
    const read = async () => {
      await p.page.waitForTimeout(100);
      return p.page.evaluate(() => [blipTouchDown(0), Number(blipTouchPos(0, 0).toFixed(1))]);
    };
    await p.down(1, x + w * 0.2, y + h / 2);
    await p.down(2, x + w * 0.8, y + h / 2);
    assert.deepEqual(await read(), [2, 0.2], 'a second finger landing is ignored');
    await p.up(2);
    await p.move(1, x + w * 0.4, y + h / 2);
    assert.deepEqual(await read(), [2, 0.4], 'the first finger still steers after the second lifts');
  });
}

test('the end of a touch wakes a sleeping audio context, as iOS requires', async (t) => {
  const p = await phone(t, 'serpent');
  await p.page.evaluate(() => {
    window.__resumes = 0;
    Howler.volume();
    Howler.ctx.resume = () => { window.__resumes++; return Promise.resolve(); };
    Object.defineProperty(Howler.ctx, 'state', { value: 'suspended', configurable: true });
  });
  await p.page.evaluate(() => document.dispatchEvent(new Event('touchend', { bubbles: true })));
  assert.equal(await p.page.evaluate(() => window.__resumes), 1);
});

test('on a PC a click on the coin wall inserts a coin and leaves the game focused', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 800 } });
  await page.goto(`${origin}/meteors/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), null, { timeout: 60000 });
  await page.waitForTimeout(1500);
  assert.equal(await page.evaluate(() => document.getElementById('need-coin-overlay').classList.contains('visible')), true);
  await page.mouse.click(640, 300);
  await page.waitForFunction(() => !document.getElementById('need-coin-overlay').classList.contains('visible'));
  assert.equal(await page.evaluate(() => document.activeElement.id), 'glcanvas');
});

const SLUGS = ['serpent', 'bouncer', 'rally', 'galactic_defender', 'meteors', 'sky_raider', 'brawler', 'bubbler', 'adder'];
for (const [label, phoneish] of [['a PC', false], ['a phone', true]]) {
  test(`on ${label} the controls card stays away until its button is pressed, then shows every control`, async (t) => {
    const size = phoneish ? { width: 390, height: 844 } : { width: 1280, height: 800 };
    const { page, origin } = await openPage(t, 'chromium', { hasTouch: phoneish, viewport: size });
    if (phoneish) {
      const cdp = await page.context().newCDPSession(page);
      await cdp.send('Emulation.setDeviceMetricsOverride', { ...size, deviceScaleFactor: 2, mobile: true });
      await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
    }
    const failures = [];
    for (const slug of SLUGS) {
      await page.goto(`${origin}/${slug}/index.html`);
      await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), null, { timeout: 60000 });
      await page.waitForTimeout(600);
      if (await page.evaluate(() => document.documentElement.hasAttribute('data-card') || getComputedStyle(document.getElementById('controls-card')).visibility !== 'hidden')) {
        failures.push(`${slug}: the card is out before it is asked for`);
      }
      await page.evaluate(() => {
        window.__whirrs = [];
        const original = window.playControlsCard;
        window.playControlsCard = (out) => { window.__whirrs.push(out); original(out); };
      });
      await page.click('#card-button');
      await page.waitForTimeout(900);
      const card = await page.evaluate((viewport) => {
        const box = document.getElementById('controls-card').getBoundingClientRect();
        return {
          labels: [...document.querySelectorAll('#controls-card .pc-label')].map((l) => l.textContent),
          inside: box.left >= 0 && box.right <= viewport.width && box.top >= 0 && box.bottom <= viewport.height,
          focus: document.activeElement.id,
        };
      }, size);
      if (!card.labels.length) failures.push(`${slug}: no controls on the card`);
      if (!card.inside) failures.push(`${slug}: the card leaves the screen`);
      if (card.focus !== 'glcanvas') failures.push(`${slug}: the game lost the keyboard`);
      await page.keyboard.press('Escape');
      await page.waitForTimeout(300);
      if ((await page.evaluate(() => window.__whirrs.join())) !== 'true,false') failures.push(`${slug}: the arm whirs once out and once back`);
      if (await page.evaluate(() => document.documentElement.hasAttribute('data-card'))) failures.push(`${slug}: Escape did not put the card away`);
      await page.click('#card-button');
      await page.waitForTimeout(700);
      await page.mouse.click(size.width / 2, size.height / 2);
      await page.waitForTimeout(300);
      if (await page.evaluate(() => document.documentElement.hasAttribute('data-card'))) failures.push(`${slug}: a click outside did not put the card away`);
    }
    assert.deepEqual(failures, []);
  });
}
