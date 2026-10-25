import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';
import { attractMask, imageDifference } from './lib/arcade-room.mjs';

const orientations = [
  { name: 'portrait', width: 390, height: 844 },
  { name: 'landscape', width: 844, height: 390 },
];

for (const engine of ['webkit', 'chromium']) {
  for (const orientation of orientations) {
    for (const format of ['webp', 'png-after-error', 'png-unsupported-type']) {
      test(`attract frames paint on iPhone 14-sized ${orientation.name}, ${format}, in ${engine}`, async (t) => {
        const { page, origin } = await openPage(t, engine, { viewport: { width: orientation.width, height: orientation.height }, hasTouch: true });
        const requests = [];
        page.on('request', (r) => { if (/\/attract\.(webp|png)$/.test(r.url())) requests.push(r.url()); });
        if (format === 'png-after-error') await page.route('**/attract.webp', (route) => route.abort());
        if (format === 'png-unsupported-type') await page.addInitScript(() => {
          // Exercise declarative unsupported-format selection without pretending to disable WebKit's codec.
          const type = Object.getOwnPropertyDescriptor(HTMLSourceElement.prototype, 'type');
          Object.defineProperty(HTMLSourceElement.prototype, 'type', {
            get: type.get,
            set(value) { type.set.call(this, value === 'image/webp' ? 'image/x-blip-unsupported' : value); },
          });
        });
        await page.goto(`${origin}/index.html?room=full`);
        await page.waitForSelector('.card-focused[data-loop-ready]');
        const strip = page.locator('.card-focused .card-loop-strip');
        const selected = await strip.evaluate((image) => ({ src: image.currentSrc }));
        assert.ok(selected.src.endsWith(format === 'webp' ? '.webp' : '.png'), selected.src);
        if (format === 'webp') assert.ok(!requests.some((r) => r.endsWith('.png')), 'supporting browsers fetch only WebP');
        if (format === 'png-unsupported-type') assert.ok(!requests.some((r) => r.endsWith('.webp')), 'an unsupported source type is skipped');
        const screen = page.locator('.card-focused .card-loop-frame');
        assert.equal(await screen.evaluate((element) => element.getAnimations()[0].effect.getKeyframes()[0].backgroundPositionY), '0%');
        const mask = await attractMask(page);
        const shots = [];
        for (const time of [0, 1000, 2000]) {
          await screen.evaluate((image, time) => {
            const animation = image.getAnimations()[0]; animation.pause(); animation.currentTime = time;
          }, time);
          await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
          const shot = await screen.screenshot();
          const dir = new URL('artifacts/attract/', import.meta.url);
          await mkdir(dir, { recursive: true });
          await writeFile(new URL(`${engine}-${orientation.name}-${format}-${time}.png`, dir), shot);
          shots.push(shot);
        }
        assert.ok((await imageDifference(page, shots[0], shots[1], 12, mask)).fraction > 0 &&
          (await imageDifference(page, shots[1], shots[2], 12, mask)).fraction > 0,
        'successive game frames paint differently with the focused overlay present');
      });
    }
  }

  test(`desktop focused screens keep playing with their overlay, hover and close view in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 1280, height: 800 } });
    await page.emulateMedia({ reducedMotion: 'no-preference' });
    await page.goto(`${origin}/index.html?room=full`);
    const games = await page.locator('#game-grid .card').evaluateAll((cards) => cards.map((card) => card.dataset.machine));
    const dir = new URL('artifacts/attract/', import.meta.url), report = [];
    await mkdir(dir, { recursive: true });
    for (const game of games) {
      await page.waitForFunction((game) => document.querySelector('.card-focused[data-loop-ready]')?.dataset.machine === game &&
        !document.getElementById('game-grid').hasAttribute('data-aisle-moving'), game);
      const card = page.locator('.card-focused'), screen = card.locator('.card-loop-frame');
      await page.mouse.move(0, 0); await card.focus();
      for (const state of ['focused', 'hover', 'close']) {
        if (state === 'hover') {
          const r = await screen.boundingBox();
          await page.mouse.move(r.x + r.width / 2, r.y + r.height / 2);
          assert.equal(await card.evaluate((element) => element.matches(':hover')), true);
        }
        if (state === 'close') {
          await page.keyboard.press('ArrowUp');
          await page.waitForFunction(() => document.getElementById('game-grid').hasAttribute('data-stepped-up') &&
            !document.getElementById('game-grid').hasAttribute('data-aisle-moving'));
        }
        const mask = await attractMask(page), first = await screen.screenshot();
        await page.waitForTimeout(1200);
        const second = await screen.screenshot();
        await writeFile(new URL(`${engine}-desktop-${game}-${state}.png`, dir), second);
        const difference = await imageDifference(page, first, second, 12, mask);
        assert.ok(difference.fraction > 0, `${game}, ${state}: game pixels change while the overlay is present`);
        report.push({ game, state, difference });
      }
      await page.keyboard.press('ArrowDown');
      await page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'));
      await page.keyboard.press('ArrowRight');
    }
    await writeFile(new URL(`${engine}-desktop-focus.json`, dir), JSON.stringify(report, null, 2));
  });

  test(`failed WebP and PNG attract images retain Brawler's still in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 390, height: 844 }, hasTouch: true });
    await page.route(/\/brawler\/attract\.(webp|png)$/, (route) => route.abort());
    await page.goto(`${origin}/index.html`);
    await page.waitForSelector('.card-focused[data-loop-ready]');
    const fallback = await page.locator('.card[data-machine="brawler"]').evaluate((card) => ({
      ready: card.hasAttribute('data-loop-ready'), still: getComputedStyle(card.querySelector('.card-shot')).visibility,
      loop: getComputedStyle(card.querySelector('.card-loop')).visibility,
    }));
    assert.deepEqual(fallback, { ready: false, still: 'visible', loop: 'hidden' });
  });

  test(`every game's PNG fallback paints changing frames in both orientations in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 390, height: 844 }, hasTouch: true });
    await page.route('**/attract.webp', (route) => route.abort());
    await page.goto(`${origin}/index.html?room=full`);
    const dir = new URL('artifacts/attract/', import.meta.url), report = [];
    const games = await page.locator('#game-grid .card').evaluateAll((cards) => cards.map((card) => card.dataset.machine));
    await mkdir(dir, { recursive: true });
    for (const orientation of orientations) {
      await page.setViewportSize({ width: orientation.width, height: orientation.height });
      for (const game of games) {
        await page.waitForFunction((game) => !document.getElementById('game-grid').hasAttribute('data-aisle-moving') &&
          document.querySelector('.card-focused[data-loop-ready]')?.dataset.machine === game, game);
        const card = page.locator('.card-focused'), strip = card.locator('.card-loop-strip');
        const machine = await card.getAttribute('data-machine');
        const source = await strip.evaluate((image) => image.currentSrc);
        assert.ok(source.endsWith(`/${machine}/attract.png`), source);
        const shots = [];
        const mask = await attractMask(page);
        for (const time of [0, 2000]) {
          await card.locator('.card-loop-frame').evaluate((image, time) => {
            const animation = image.getAnimations()[0]; animation.pause(); animation.currentTime = time;
          }, time);
          await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
          const shot = await card.locator('.card-loop-frame').screenshot();
          await writeFile(new URL(`${engine}-${orientation.name}-png-${machine}-${time}.png`, dir), shot);
          shots.push(shot);
        }
        const difference = await imageDifference(page, shots[0], shots[1], 12, mask);
        assert.ok(difference.fraction > 0, `${orientation.name}, ${machine}: ${JSON.stringify(difference)}`);
        report.push({ orientation: orientation.name, machine, source, difference });
        await page.keyboard.press('ArrowRight');
      }
    }
    await writeFile(new URL(`${engine}-png-all-games.json`, dir), JSON.stringify(report, null, 2));
  });

  test(`attract playback waits for decoded pixels in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 390, height: 844 }, hasTouch: true });
    await page.addInitScript(() => {
      const decode = HTMLImageElement.prototype.decode;
      HTMLImageElement.prototype.decode = function () {
        const ready = decode.call(this);
        return this.classList.contains('card-loop-strip') && this.currentSrc.includes('/rally/attract.')
          ? ready.then(() => new Promise((resolve) => { window.__releaseAttractDecode = resolve; })) : ready;
      };
    });
    await page.goto(`${origin}/index.html`);
    await page.waitForFunction(() => typeof window.__releaseAttractDecode === 'function');
    const state = () => page.locator('.card-focused').evaluate((card) => ({ ready: card.hasAttribute('data-loop-ready'),
      playback: card.querySelector('.card-loop-frame').getAnimations()[0].playState }));
    assert.deepEqual(await state(), { ready: false, playback: 'paused' });
    await page.evaluate(() => window.__releaseAttractDecode());
    await page.waitForSelector('.card-focused[data-loop-ready]');
    assert.deepEqual(await state(), { ready: true, playback: 'running' });
  });
}
