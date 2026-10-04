import test from 'node:test';
import assert from 'node:assert/strict';
import { startPage } from './lib/harness.mjs';

test('a quarter fits the horizontal slot and clinks into the coin box', async (t) => {
  const handle = await startPage('chromium', {
    hasTouch: true, viewport: { width: 390, height: 844 },
  });
  t.after(handle.close);
  const { page, origin } = handle;
  await page.goto(`${origin}/index.html`);
  await page.waitForSelector('.deck-coin-slot #coin-drop-anim');

  const hardware = await page.evaluate(() => {
    const box = document.querySelector('.deck-coin-slot');
    const bar = document.querySelector('.kiosk-bar');
    const deck = bar.querySelector('.deck-panel');
    const boxRect = box.getBoundingClientRect();
    const mouth = box.querySelector('.coin-mouth').getBoundingClientRect();
    const label = box.querySelector('.coin-instruction');
    const size = parseFloat(getComputedStyle(document.querySelector('#coin-drop-anim'), '::before').width);
    const button = [...document.querySelectorAll('.arcade-btn:not(.spare),.snes-btn:not(.spare)')]
      .map((element) => element.getBoundingClientRect()).find((rect) => rect.width > 0);
    return {
      boxHeight: box.offsetHeight,
      barHeight: bar.clientHeight,
      boxWidth: boxRect.width,
      deckRight: deck.offsetLeft + deck.offsetWidth,
      boxLeft: box.offsetLeft,
      deckBottom: deck.offsetTop + deck.offsetHeight,
      boxBottom: box.offsetTop + box.offsetHeight,
      boxPlane: getComputedStyle(box).transform,
      deckPlane: getComputedStyle(bar.querySelector('.deck-panel')).transform,
      sharedPerspective: getComputedStyle(bar).perspective,
      sharedPerspectiveOrigin: getComputedStyle(bar).perspectiveOrigin,
      perspectiveOriginX: bar.clientWidth / 2,
      perspectiveOriginY: bar.clientHeight,
      mouthWidth: mouth.width,
      mouthHeight: mouth.height,
      coinSize: size,
      labelWidth: label.clientWidth,
      labelScrollWidth: label.scrollWidth,
      buttonSize: button && Math.max(button.width, button.height),
    };
  });
  assert.ok(Math.abs((hardware.barHeight - hardware.boxHeight) - 4) <= 1,
    'the coinbox is slightly shorter at the top');
  assert.ok(Math.abs(hardware.deckBottom - hardware.boxBottom) <= 1,
    'the coinbox bottom edge aligns with the joystick and pad deck');
  assert.ok(hardware.boxWidth >= 80 && hardware.boxHeight >= 56,
    'the coin box stays easy to tap');
  assert.ok(Math.abs(hardware.deckRight - hardware.boxLeft) <= 1,
    'the coinbox side edge meets the joystick and pad deck edge');
  assert.equal(hardware.boxPlane, hardware.deckPlane,
    'the coinbox and controls share the same tilt');
  assert.equal(parseFloat(hardware.sharedPerspective), 1.8 * 390,
    'the deck and coinbox get perspective from the same physical viewpoint');
  const perspectiveOrigin = hardware.sharedPerspectiveOrigin.split(' ').map(parseFloat);
  assert.ok(Math.abs(perspectiveOrigin[0] - hardware.perspectiveOriginX) < 1
    && Math.abs(perspectiveOrigin[1] - hardware.perspectiveOriginY) < 1,
    'the deck and coinbox share one vanishing point along their joined edge');
  assert.ok(hardware.mouthWidth > hardware.mouthHeight * 4,
    `the coin opening is unmistakably horizontal (${hardware.mouthWidth}x${hardware.mouthHeight})`);
  assert.ok(hardware.coinSize <= hardware.mouthWidth,
    'the quarter fits through the opening');
  assert.ok(Math.abs(hardware.coinSize - hardware.buttonSize) < 12,
    'the quarter is about the size of a deck button');
  assert.ok(hardware.labelScrollWidth <= hardware.labelWidth,
    'INSERT COIN stays inside the metal plate');

  await page.evaluate(() => {
    window.__coinToneOffsets = [];
    const Audio = window.AudioContext || window.webkitAudioContext;
    const create = Audio.prototype.createOscillator;
    Audio.prototype.createOscillator = function () {
      const context = this, oscillator = create.call(this), start = oscillator.start.bind(oscillator);
      oscillator.start = function (when) {
        if (Number.isFinite(when)) window.__coinToneOffsets.push(when - context.currentTime);
        return start(when);
      };
      return oscillator;
    };
  });
  await page.locator('.deck-coin-slot').tap();
  assert.equal(await page.evaluate(() => getCoins()), 1, 'the touch inserts one credit');

  await page.waitForTimeout(560);
  const insertion = await page.evaluate(() => {
    const coin = document.querySelector('#coin-drop-anim');
    const style = getComputedStyle(coin, '::before');
    const size = parseFloat(style.width);
    return {
      opacity: Number(style.opacity),
      name: style.animationName,
      projectedHeight: size * Math.abs(new DOMMatrixReadOnly(style.transform).m22),
      slotHeight: coin.parentElement.querySelector('.coin-mouth').getBoundingClientRect().height,
    };
  });
  assert.equal(insertion.name, 'coin-drop', 'the coin is still moving at the slot');
  assert.ok(insertion.opacity > 0, 'the coin remains visible as it enters');
  assert.ok(insertion.projectedHeight <= insertion.slotHeight,
    `the tilted coin edge fits inside the horizontal opening (${insertion.projectedHeight}px in ${insertion.slotHeight}px)`);
  const clinks = await page.evaluate(() => window.__coinToneOffsets);
  assert.ok(clinks.some((offset) => offset > 0.6 && offset < 0.78),
    'the coin-box clatter follows the coin through the slot');

  await page.waitForTimeout(400);
  assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector('#coin-drop-anim'), '::before').opacity), '0',
    'the coin disappears into the collection box');

  await page.setViewportSize({ width: 844, height: 390 });
  await page.waitForFunction(() => document.documentElement.dataset.layout === 'landscape');
  const sideways = await page.locator('.deck-coin-slot').evaluate((box) => {
    const deck = box.closest('.kiosk-bar').querySelector('.deck-panel');
    return { height: box.getBoundingClientRect().height,
      deckRight: deck.offsetLeft + deck.offsetWidth, boxLeft: box.offsetLeft };
  });
  assert.equal(sideways.height, 62, 'the coin box stays aligned with the compact sideways deck');
  assert.ok(Math.abs(sideways.deckRight - sideways.boxLeft) <= 1,
    'the sideways deck and coinbox share a flush side edge');
  await page.locator('.deck-coin-slot').tap();
  assert.equal(await page.evaluate(() => getCoins()), 2, 'the coin box also accepts a sideways touch');

  await page.setViewportSize({ width: 320, height: 568 });
  await page.waitForFunction(() => document.documentElement.dataset.layout === 'upright');
  const compactFit = await page.locator('.deck-coin-slot').evaluate((box) => {
    const label = box.querySelector('.coin-instruction');
    const deck = box.closest('.kiosk-bar').querySelector('.deck-panel');
    return { width: box.getBoundingClientRect().width, text: label.scrollWidth <= label.clientWidth,
      deckRight: deck.offsetLeft + deck.offsetWidth, boxLeft: box.offsetLeft };
  });
  assert.ok(compactFit.width >= 80 && compactFit.text,
    'the coin plate and its instruction fit on a narrow touch screen');
  assert.ok(Math.abs(compactFit.deckRight - compactFit.boxLeft) <= 1,
    'the narrow deck and coinbox share a flush side edge');
  await page.locator('.deck-coin-slot').tap();
  assert.equal(await page.evaluate(() => getCoins()), 3, 'the narrow touch layout accepts a coin');

  await page.goto(`${origin}/bouncer/index.html`, { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.deck-coin-slot #coin-drop-anim');
  const gamePageAligned = await page.evaluate(() => {
    const box = document.querySelector('.deck-coin-slot');
    const bar = document.querySelector('.kiosk-bar');
    const deck = bar.querySelector('.deck-panel');
    const barStyle = getComputedStyle(bar), deckStyle = getComputedStyle(deck);
    return { box: box.offsetHeight, bar: bar.clientHeight,
      deckBottom: deck.offsetTop + deck.offsetHeight, boxBottom: box.offsetTop + box.offsetHeight,
      deckLeft: deck.offsetLeft, deckWidth: deck.offsetWidth, deckRight: deck.offsetLeft + deck.offsetWidth,
      boxLeft: box.offsetLeft, boxWidth: box.offsetWidth, right: deckStyle.right,
      barWidth: bar.offsetWidth, barPadding: barStyle.paddingRight };
  });
  assert.ok(Math.abs(gamePageAligned.bar - gamePageAligned.box - 4) <= 1
    && Math.abs(gamePageAligned.deckBottom - gamePageAligned.boxBottom) <= 1
    && Math.abs(gamePageAligned.deckRight - gamePageAligned.boxLeft) <= 1,
    `the game page coin box follows its control deck height (${JSON.stringify(gamePageAligned)})`);
  await page.locator('.deck-coin-slot').tap();
  assert.equal(await page.evaluate(() => getCoins()), 4, 'the game page accepts touch coin insertion');
});
