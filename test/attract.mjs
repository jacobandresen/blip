import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';
import { imageDifference } from './lib/arcade-room.mjs';

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
        await page.addStyleTag({ content: '.machine-attract {visibility:hidden!important}' });
        const strip = page.locator('.card-focused .card-loop-strip');
        const selected = await strip.evaluate((image) => ({ src: image.currentSrc, start: image.getAnimations()[0].effect.getKeyframes()[0].transform }));
        assert.ok(selected.src.endsWith(format === 'webp' ? '.webp' : '.png'), selected.src);
        assert.equal(selected.start, 'translateY(0px)', 'the clipped animation starts with an explicit transform');
        if (format === 'webp') assert.ok(!requests.some((r) => r.endsWith('.png')), 'supporting browsers fetch only WebP');
        if (format === 'png-unsupported-type') assert.ok(!requests.some((r) => r.endsWith('.webp')), 'an unsupported source type is skipped');
        const screen = page.locator('.card-focused .card-loop-frame');
        const shots = [];
        for (const time of [0, 1000, 2000]) {
          await strip.evaluate((image, time) => {
            const animation = image.getAnimations()[0]; animation.pause(); animation.currentTime = time;
          }, time);
          await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
          const shot = await screen.screenshot();
          const dir = new URL('artifacts/attract/', import.meta.url);
          await mkdir(dir, { recursive: true });
          await writeFile(new URL(`${engine}-${orientation.name}-${format}-${time}.png`, dir), shot);
          shots.push(shot);
        }
        assert.ok(!shots[0].equals(shots[1]) && !shots[1].equals(shots[2]), 'successive attract frames paint different game pictures');
      });
    }
  }

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
    await page.addStyleTag({ content: '.machine-attract {visibility:hidden!important}' });
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
        for (const time of [0, 2000]) {
          await strip.evaluate((image, time) => {
            const animation = image.getAnimations()[0]; animation.pause(); animation.currentTime = time;
          }, time);
          await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
          const shot = await card.locator('.card-loop-frame').screenshot();
          await writeFile(new URL(`${engine}-${orientation.name}-png-${machine}-${time}.png`, dir), shot);
          shots.push(shot);
        }
        const difference = await imageDifference(page, shots[0], shots[1], 12);
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
      playback: card.querySelector('.card-loop-strip').getAnimations()[0].playState }));
    assert.deepEqual(await state(), { ready: false, playback: 'paused' });
    await page.evaluate(() => window.__releaseAttractDecode());
    await page.waitForSelector('.card-focused[data-loop-ready]');
    assert.deepEqual(await state(), { ready: true, playback: 'running' });
  });
}
