import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const touch of [false, true]) {
  test(`tape pickup and seating preserve the cassette on ${touch ? 'touch' : 'desktop'}`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', {
      hasTouch: touch, viewport: touch ? { width: 390, height: 844 } : { width: 1000, height: 900 },
    });
    await page.route('**/*.wasm', () => {});
    await page.goto(`${origin}/basic.html`);
    const tape = page.locator('.game-tape[data-tape="ADDER"]:not(.tape-placeholder)');
    const original = await tape.elementHandle();
    const buttons = await page.locator('.tape-buttons > span').evaluateAll((keys) => keys.map((key) => {
      const r = key.getBoundingClientRect();
      return { height: r.height, visible: Boolean(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('.tape-buttons')) };
    }));
    assert.equal(buttons.length, 5);
    assert.ok(buttons.every((key) => key.height >= 26 && key.visible), JSON.stringify(buttons));
    if (touch) await tape.tap({ position: { x: 12, y: 18 } });
    else await tape.click({ position: { x: 100, y: 24 } });
    await page.waitForSelector('.game-tape-flight');
    assert.equal(await tape.evaluate((el, initial) => el === initial && !document.querySelector('#tape-pile').contains(el), original), true);
    await tape.evaluate((el) => {
      for (const animation of el.getAnimations({ subtree: true })) { animation.pause(); animation.currentTime = 1500; }
    });
    await page.screenshot({ path: `/tmp/blip-tape-${touch ? 'touch' : 'desktop'}-pickup.png` });
    await tape.evaluate((el) => el.getAnimations({ subtree: true }).forEach((animation) => animation.play()));
    await page.waitForSelector('.game-tape-inserting');
    const insertion = await tape.evaluate((el) => {
      const lid = document.querySelector('.deck-lid');
      return { open: document.body.hasAttribute('data-deck-open'), beneath: +getComputedStyle(el).zIndex < +getComputedStyle(lid).zIndex, lidMoving: lid.getAnimations().some((animation) => animation.playState === 'running') };
    });
    assert.deepEqual(insertion, { open: true, beneath: true, lidMoving: false });
    await page.waitForFunction(() => document.body.hasAttribute('data-loading-tape'));
    assert.equal(await tape.evaluate((el, initial) => el === initial, original), true);
    const seated = await tape.evaluate((el) => {
      const tape = el.getBoundingClientRect(), bay = el.parentElement.getBoundingClientRect();
      return { inside: tape.left >= bay.left && tape.right <= bay.right && tape.top >= bay.top && tape.bottom <= bay.bottom, closed: !document.body.hasAttribute('data-deck-open') };
    });
    assert.deepEqual(seated, { inside: true, closed: true });
    await page.screenshot({ path: `/tmp/blip-tape-${touch ? 'touch' : 'desktop'}-loaded.png` });
  });
}
