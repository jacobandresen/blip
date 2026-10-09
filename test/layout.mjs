// Nothing overlaps and nothing is pushed off screen: every page of the cabinet on a spread of real
// devices, with their notches, in portrait and landscape, with each controller.
//
//   node --test --test-concurrency=1 test/layout.mjs        (after ./build_web.sh)
//   LAYOUT=iPad node --test test/layout.mjs                  (only the devices whose name has "iPad")

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

// CSS pixels; the insets are what the notch, the island and the home bar take.
const DEVICES = [
  { name: 'iPhone SE', w: 320, h: 568 },
  { name: 'iPhone 8', w: 375, h: 667 },
  { name: 'iPhone 12 mini', w: 360, h: 780, top: 50, bottom: 34 },
  { name: 'iPhone 14', w: 390, h: 844, top: 47, bottom: 34 },
  { name: 'iPhone 14 Pro', w: 393, h: 852, top: 59, bottom: 34 },
  { name: 'iPhone 14 Pro Max', w: 430, h: 932, top: 59, bottom: 34 },
  { name: 'Pixel 5', w: 393, h: 851 },
  { name: 'Galaxy S20', w: 360, h: 800 },
  { name: 'small Android', w: 320, h: 640 },
  { name: 'iPhone 14 landscape', w: 844, h: 390, left: 47, right: 47, bottom: 21 },
  { name: 'iPhone SE landscape', w: 667, h: 375 },
  { name: 'Pixel 5 landscape', w: 851, h: 393 },
  { name: 'iPad mini', w: 744, h: 1133 },
  { name: 'iPad Air', w: 820, h: 1180 },
  { name: 'iPad Pro', w: 1024, h: 1366 },
  { name: 'iPad landscape', w: 1180, h: 820 },
  { name: 'laptop 1280x720', w: 1280, h: 720, desktop: true },
  { name: 'laptop 1366x768', w: 1366, h: 768, desktop: true },
  { name: 'desktop 1440x900', w: 1440, h: 900, desktop: true },
  { name: 'desktop 1920x1080', w: 1920, h: 1080, desktop: true },
  { name: 'desktop 2560x1440', w: 2560, h: 1440, desktop: true },
];

const TWO_SEATS = ['rally', 'brawler', 'bubbler', 'adder'];
const GAMES = ['serpent', 'bouncer', 'rally', 'galactic_defender', 'meteors', 'sky_raider', 'brawler', 'bubbler', 'adder'];

// The pieces of interface that must each have their own space.
const PIECES = {
  logo: '.blip-logo',
  title: '#marquee-name',
  manual: '.manual-pocket',
  cardButton: '#card-button',
  lamp: '.blip-power-indicator',
  receiver: '.jukebox-receiver',
  coin: '.deck-coin-slot',
  dpad: '.snes-dpad',
  button: '.snes-btn, .arcade-btn',
  stick: '.stick-base',
  strip: '.ts-glass',
  join: '.ts-join',
  toggle: '#control-toggle',
  creditLamp: '.deck-credit-lamp, .deck-second-lamp',
};

// The visible part of a piece: round buttons, the cross and the stick's boot do not fill their boxes.
const CORE = { dpad: 0.2, button: 0.14, stick: 0.25, creditLamp: 0.2 };

// What a thumb needs, in CSS pixels. The deck is tilted back, so a cap is a little shorter than it is wide.
const THUMB = { button: { w: 38, h: 34 }, dpad: { w: 52, h: 46 }, stick: { w: 52, h: 46 } };
const APART = 4;

// Where the controls must not reach: under the home bar.
const CONTROLS = ['coin', 'dpad', 'button', 'stick', 'strip', 'join', 'toggle'];

function collect(core) {
  const visible = (el) => {
    const style = getComputedStyle(el), r = el.getBoundingClientRect();
    return r.width > 2 && r.height > 2 && style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) > 0.05;
  };
  const found = [];
  for (const [key, selector] of Object.entries(window.__pieces)) {
    document.querySelectorAll(selector).forEach((el, index) => {
      if (!visible(el)) return;
      const r = el.getBoundingClientRect(), cut = core[key] || 0;
      const dx = r.width * cut, dy = r.height * cut;
      found.push({ name: `${key}${index ? '#' + index : ''}`, key, left: r.left + dx, top: r.top + dy, right: r.right - dx, bottom: r.bottom - dy,
        whole: { left: r.left, top: r.top, right: r.right, bottom: r.bottom }, el });
    });
  }
  const inside = (a, b) => a.left >= b.left - 1 && a.right <= b.right + 1 && a.top >= b.top - 1 && a.bottom <= b.bottom + 1;
  const overlaps = [];
  for (let i = 0; i < found.length; i++) {
    for (let j = i + 1; j < found.length; j++) {
      const a = found[i], b = found[j];
      if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
      if (inside(a, b) || inside(b, a)) { if (a.key !== b.key) overlaps.push(`${a.name} lies wholly over ${b.name}`); continue; }
      const across = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const down = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (across > 3 && down > 3) overlaps.push(`${a.name} overlaps ${b.name} by ${Math.round(across)}x${Math.round(down)}px`);
    }
  }
  const clipped = [];
  for (const selector of ['.coin-instruction', '#marquee-name']) {
    document.querySelectorAll(selector).forEach((el) => {
      const style = getComputedStyle(el);
      if (style.visibility !== 'hidden' && style.display !== 'none' && el.scrollWidth > el.clientWidth + 1) clipped.push(`${selector} is cut off (${el.scrollWidth}px of text in ${el.clientWidth}px)`);
    });
  }
  const thumbs = [];
  const controls = found.filter((f) => ['button', 'dpad', 'stick'].includes(f.key));
  for (let i = 0; i < controls.length; i++) {
    for (let j = i + 1; j < controls.length; j++) {
      const a = controls[i], b = controls[j];
      const gap = Math.hypot(Math.max(a.left - b.right, b.left - a.right, 0), Math.max(a.top - b.bottom, b.top - a.bottom, 0));
      const touching = Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0;
      if (!touching && gap < 4) thumbs.push(`${a.name} and ${b.name} are only ${gap.toFixed(1)}px apart`);
    }
  }
  return {
    clipped,
    thumbs,
    overlaps,
    boxes: found.map(({ name, key, left, top, right, bottom, whole }) => ({ name, key, left, top, right, bottom, whole })),
    sizes: found.filter((f) => ['button', 'dpad', 'stick'].includes(f.key)).map((f) => ({ name: f.name, key: f.key, w: f.whole.right - f.whole.left, h: f.whole.bottom - f.whole.top })),
    sideways: document.documentElement.scrollWidth - innerWidth,
  };
}

async function open(t, device) {
  const touch = !device.desktop;
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: touch, viewport: { width: device.w, height: device.h } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: device.w, height: device.h, deviceScaleFactor: 2, mobile: touch });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: touch });
  const insets = { top: device.top || 0, bottom: device.bottom || 0, left: device.left || 0, right: device.right || 0 };
  await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets });
  await page.addInitScript((pieces) => { window.__pieces = pieces; }, PIECES);
  return { page, origin, insets };
}

async function check(page, device, insets, label, failures) {
  const covered = await page.evaluate(() => {
    const onTop = (el) => {
      if (!el) return null;
      const r = el.getBoundingClientRect();
      const hit = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return hit && !hit.closest('.card, #glcanvas, #need-coin-overlay, #loader, .blip-hs-modal, #rotate-hint') ? hit.tagName + '.' + String(hit.className).slice(0, 30) : null;
    };
    return onTop(document.querySelector('.card-focused')) || onTop(document.getElementById('glcanvas'));
  });
  if (covered) failures.push(`${label}: the middle of the picture is covered by ${covered}`);
  const result = await page.evaluate(collect, CORE);
  for (const line of result.overlaps) failures.push(`${label}: ${line}`);
  // Two players share 320px on the smallest phones: each gets about 110px, so their controls may be small and close, though never overlapping.
  const squeezed = device.w < 340 && label.startsWith('brawler');
  if (!squeezed) for (const line of result.thumbs) failures.push(`${label}: ${line}`);
  for (const line of result.clipped) failures.push(`${label}: ${line}`);
  if (!device.desktop && !squeezed) {
    for (const size of result.sizes) {
      const need = THUMB[size.key];
      if (size.w < need.w || size.h < need.h) failures.push(`${label}: ${size.name} is ${Math.round(size.w)}x${Math.round(size.h)}px, too small for a thumb`);
    }
  }
  if (result.sideways > 0) failures.push(`${label}: the page is ${result.sideways}px wider than the screen`);
  const edge = { left: insets.left, right: device.w - insets.right, bottom: device.h - insets.bottom };
  for (const box of result.boxes) {
    const w = box.whole;
    const reaches = w.left < -1 || w.right > device.w + 1 || w.top < -1 || w.bottom > device.h + 1;
    if (reaches) failures.push(`${label}: ${box.name} leaves the screen (${Math.round(w.left)},${Math.round(w.top)} to ${Math.round(w.right)},${Math.round(w.bottom)})`);
    if (CONTROLS.includes(box.key) && !device.desktop && box.key !== 'coin' && box.bottom > edge.bottom + 2) {
      failures.push(`${label}: ${box.name} sits under the home bar (${Math.round(box.bottom)} past ${edge.bottom})`);
    }
    if (box.key !== 'receiver' && (box.left < edge.left - 1 || box.right > edge.right + 1) && insets.left) {
      failures.push(`${label}: ${box.name} sits under the notch`);
    }
  }
}

const only = process.env.LAYOUT;
for (const device of DEVICES.filter((d) => !only || d.name.includes(only))) {
  test(`nothing overlaps or leaves the screen on ${device.name} (${device.w}x${device.h})`, async (t) => {
    const { page, origin, insets } = await open(t, device);
    const failures = [];
    const settle = () => page.waitForTimeout(1800);

    await page.goto(`${origin}/index.html`);
    await settle();
    await check(page, device, insets, 'cabinet', failures);

    await page.evaluate(() => sessionStorage.setItem('blip-loaded-card',
      JSON.stringify({ slug: 'bouncer', name: 'BOUNCER', code: 'A2', art: 'bouncer/screenshot.png' })));
    await page.goto(`${origin}/index.html`);
    await settle();
    await check(page, device, insets, 'cabinet with a ROM seated', failures);
    await page.evaluate(() => sessionStorage.clear());

    const controllers = device.desktop ? [null] : [null, 'stick', 'pad'];
    for (const slug of GAMES) {
      for (const controller of controllers) {
        await page.addInitScript((mode) => { if (mode) localStorage.setItem('blip-controls', mode); else localStorage.removeItem('blip-controls'); }, controller);
        await page.goto(`${origin}/${slug}/index.html`);
        await page.waitForFunction(() => typeof BlipController === 'object', null, { timeout: 30000 });
        await settle();
        await check(page, device, insets, `${slug}${controller ? ' with the ' + controller : ''}`, failures);
        if (TWO_SEATS.includes(slug)) {
          await page.evaluate(() => blipSetMode(1));
          await settle();
          await check(page, device, insets, `${slug}${controller ? ' with the ' + controller : ''}, two players`, failures);
        }
      }
    }
    assert.deepEqual(failures, []);
  });
}

for (const device of DEVICES.filter((d) => !only || d.name.includes(only))) {
  test(`the manual sits inside the screen and its buttons are clear on ${device.name} (${device.w}x${device.h})`, async (t) => {
    const { page, origin, insets } = await open(t, device);
    const failures = [];
    for (const chapter of ['about', 'controls']) {
      await page.goto(`${origin}/index.html?manual=${chapter}`);
      await page.waitForFunction(() => !document.getElementById('manual-overlay').inert && document.querySelector('.manual-page-content .manual-leaf-sheet'), null, { timeout: 30000 });
      await page.waitForTimeout(700);
      const shown = await page.evaluate(() => {
        const box = (selector) => [...document.querySelectorAll(selector)].filter((el) => {
          const style = getComputedStyle(el), r = el.getBoundingClientRect();
          return r.width > 2 && r.height > 2 && style.display !== 'none' && style.visibility !== 'hidden' && !el.closest('.manual-flip, .manual-flip-cover');
        }).map((el) => { const r = el.getBoundingClientRect(); return { left: r.left, top: r.top, right: r.right, bottom: r.bottom }; });
        return {
          book: box('.manual-open-book')[0], close: box('.manual-close')[0], tab: box('.manual-contents-tab')[0],
          prev: box('.manual-turn-prev')[0], next: box('.manual-turn-next')[0], pages: box('.manual-open-page').length,
        };
      });
      const label = `manual (${chapter})`;
      const inside = (name, b) => {
        if (!b) return;
        if (b.left < insets.left - 1 || b.right > device.w - insets.right + 1 || b.top < insets.top - 1 || b.bottom > device.h - insets.bottom + 1) {
          failures.push(`${label}: ${name} leaves the screen (${Math.round(b.left)},${Math.round(b.top)} to ${Math.round(b.right)},${Math.round(b.bottom)})`);
        }
      };
      for (const name of ['book', 'close', 'tab', 'prev', 'next']) inside(name, shown[name]);
      const apart = (a, b, names) => {
        if (!a || !b) return;
        if (Math.min(a.right, b.right) - Math.max(a.left, b.left) > 2 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 2) failures.push(`${label}: ${names} overlap`);
      };
      apart(shown.close, shown.tab, 'the close and contents tabs');
      apart(shown.prev, shown.next, 'the turn buttons');
      if (device.w > 700 && shown.pages !== 2) failures.push(`${label}: a wide book shows ${shown.pages} pages, not two`);
      if (device.w <= 700 && shown.pages !== 1) failures.push(`${label}: a narrow book shows ${shown.pages} pages, not one`);
      const sideways = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
      if (sideways > 0) failures.push(`${label}: the page is ${sideways}px wider than the screen`);
    }
    assert.deepEqual(failures, []);
  });
}

for (const device of DEVICES.filter((d) => !only || d.name.includes(only))) {
  test(`every card fits the cabinet window with its text on ${device.name} (${device.w}x${device.h})`, async (t) => {
    const { page, origin } = await open(t, device);
    const failures = [];
    await page.goto(`${origin}/index.html`);
    await page.waitForFunction(() => document.querySelector('.card-focused')?.offsetWidth > 0);
    await page.waitForTimeout(1200);
    for (let step = 0; step < 9; step++) {
      await page.waitForFunction(() => [...document.querySelectorAll('.card')].every((card) => card.getAnimations().length === 0));
      const card = await page.evaluate(() => {
        const el = document.querySelector('.card-focused');
        const box = el.getBoundingClientRect(), bezel = document.querySelector('.screen-bezel').getBoundingClientRect();
        const text = ['.card-title', '.card-genre', '.card-desc'].flatMap((selector) => {
          const range = document.createRange();
          range.selectNodeContents(el.querySelector(selector));
          return [...range.getClientRects()].filter((r) => r.width > 0);
        });
        const covering = [...document.querySelectorAll('.snes-dpad, .snes-btn, .arcade-btn, .stick-base, .deck-coin-slot')].filter((control) => {
          const c = control.getBoundingClientRect(), style = getComputedStyle(control);
          if (c.width < 3 || style.display === 'none' || style.visibility === 'hidden') return false;
          const dx = c.width * 0.18, dy = c.height * 0.18;
          return Math.min(c.right - dx, box.right) - Math.max(c.left + dx, box.left) > 4 && Math.min(c.bottom - dy, box.bottom) - Math.max(c.top + dy, box.top) > 4;
        }).map((control) => control.className.toString().split(' ')[0]);
        return {
          covered: covering.length ? [...new Set(covering)].join(' and ') : '',
          title: el.querySelector('.card-title').textContent,
          inBezel: box.left >= bezel.left - 1 && box.right <= bezel.right + 1 && box.top >= bezel.top - 1 && box.bottom <= bezel.bottom + 1,
          textOnCard: text.every((r) => r.left >= box.left - 1 && r.right <= box.right + 1 && r.top >= box.top - 1 && r.bottom <= box.bottom + 1),
          fontPx: parseFloat(getComputedStyle(el.querySelector('.card-desc')).fontSize),
        };
      });
      if (card.covered) failures.push(`${card.title}: ${card.covered} sit over the card`);
      if (!card.inBezel) failures.push(`${card.title}: the card leaves the cabinet window`);
      if (!card.textOnCard) failures.push(`${card.title}: its text runs off the card`);
      if (card.fontPx < 11) failures.push(`${card.title}: its text is only ${card.fontPx}px`);
      await page.keyboard.press('ArrowRight');
    }
    assert.deepEqual(failures, []);
  });
}

for (const device of DEVICES.filter((d) => !only || d.name.includes(only))) {
  test(`the controls card opens inside the screen and clear of the bar on ${device.name} (${device.w}x${device.h})`, async (t) => {
    const { page, origin } = await open(t, device);
    const failures = [];
    for (const slug of ['cabinet', ...GAMES]) {
      await page.goto(`${origin}/${slug === 'cabinet' ? '' : `${slug}/`}index.html`);
      if (slug === 'cabinet') await page.waitForSelector('#card-button');
      else await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), null, { timeout: 60000 });
      await page.waitForTimeout(400);
      await page.evaluate(() => document.getElementById('card-button')?.click());
      await page.waitForTimeout(900);
      const card = await page.evaluate(() => {
        const box = document.getElementById('controls-card').getBoundingClientRect();
        const bar = document.getElementById('marquee-bar')?.getBoundingClientRect();
        const groups = [...document.querySelectorAll('#controls-card .pc-group, #controls-card .cc-lines li')].map((g) => g.getBoundingClientRect());
        return {
          box: [box.left, box.top, box.right, box.bottom], bar: bar ? bar.bottom : 0,
          groupsInside: groups.length > 0 && groups.every((g) => g.left >= box.left && g.right <= box.right && g.bottom <= box.bottom),
        };
      });
      const [left, top, right, bottom] = card.box;
      if (left < (device.left || 0) || right > device.w - (device.right || 0) || bottom > device.h) failures.push(`${slug}: the card leaves the screen`);
      if (top < card.bar - 1) failures.push(`${slug}: the card sits under the top bar`);
      if (!card.groupsInside) failures.push(`${slug}: the controls run off the card`);
    }
    assert.deepEqual(failures, []);
  });
}
