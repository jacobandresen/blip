import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';

function cabinetSurfaces(card) {
  const root = card || document.documentElement;
  const css = (selector, pseudo) => getComputedStyle(root.querySelector(selector), pseudo);
  return {
    body: getComputedStyle(card || document.body).backgroundImage,
    monitor: css(card ? '.card-shot-wrap' : '#arcade-bezel').backgroundImage,
    monitorRing: css(card ? '.card-shot-wrap' : '#arcade-bezel').boxShadow,
    glass: css(card ? '.game-rom' : '.cabinet-glass', card ? '::after' : null).backgroundImage,
    speaker: css('.machine-speaker').backgroundImage,
    apron: css(card ? '.machine-controls' : '#topbar .deck-housing', card ? '::after' : '::before').backgroundImage,
    title: css(card ? '.card-title' : '#marquee-bar').backgroundImage,
  };
}

function deckHardware() {
  const root = document.documentElement;
  return { controls: root.dataset.controls, touch: root.dataset.touch || null, dials: root.hasAttribute('data-dials'),
    wear: root.dataset.deckWear,
    caps: [...document.querySelectorAll('.arcade-btn')].map(el => ({ spare: el.classList.contains('spare'), alt: el.classList.contains('alt') })),
    strip: document.querySelector('#touch-strip')?.innerHTML || null,
    panel: getComputedStyle(document.querySelector('.deck-panel')).backgroundImage };
}

for (const engine of ['chromium', 'webkit']) {
  for (const touch of [false, true]) {
    test(`each game keeps its cabinet surfaces and hardware from the overview in ${engine}, ${touch ? 'touch' : 'desktop'}`, async t => {
      const { page, origin } = await openPage(t, engine, {
        viewport: touch ? { width: 390, height: 844 } : { width: 1280, height: 800 }, hasTouch: touch,
      });
      await page.goto(`${origin}/index.html`);
      await page.waitForSelector('.card-focused[data-loop-ready]');
      const games = await page.locator('#game-grid .card').evaluateAll(cards => cards.map(card => card.dataset.machine));
      const gamePage = await page.context().newPage();
      const dir = new URL(`artifacts/machine-identity/${engine}-${touch ? 'phone' : 'desktop'}/`, import.meta.url);
      await mkdir(dir, { recursive: true });
      for (const game of games) {
        const card = page.locator(`.card[data-machine="${game}"]`);
        const overview = await card.evaluate(cabinetSurfaces);
        const frame = await (await card.locator('iframe').elementHandle()).contentFrame();
        await frame.waitForSelector('.deck-panel', { state: 'attached' });
        const hardware = await frame.evaluate(deckHardware);
        await gamePage.goto(`${origin}/${game}/index.html`);
        await gamePage.waitForSelector('html[data-machine] .machine-speaker', { state: 'attached' });
        await gamePage.waitForFunction(() => document.querySelector('#marquee-name .machine-marquee-name') &&
          (!blipHasTouch() || !BLIP_GAMES[document.documentElement.dataset.machine].touch || document.getElementById('touch-strip')));
        await gamePage.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
        // Up close the cabinet's laminate is a column in the arcade's room: every layer of it is on the page.
        const near = await gamePage.evaluate(cabinetSurfaces);
        for (const layer of overview.body.split(/, (?=linear-gradient)/))
          assert.ok(near.body.includes(layer), `${game}: the cabinet's laminate ${layer} is on its page`);
        assert.deepEqual({ ...near, body: overview.body }, overview, `${game}: physical cabinet surfaces`);
        const liveHardware = await gamePage.evaluate(deckHardware);
        // Input feedback adds inline widths and transforms only after the live shell wires the shared strip.
        if (hardware.strip) {
          const content = html => html.replace(/ style="[^"]*"/g, '');
          hardware.strip = content(hardware.strip); liveHardware.strip = content(liveHardware.strip);
        }
        assert.deepEqual(liveHardware, hardware, `${game}: controller type, caps, strip and wear`);
        if (game === 'rally' || game === 'brawler') {
          await gamePage.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
          await gamePage.screenshot({ path: new URL(`${game}.png`, dir).pathname });
        }
      }
      await writeFile(new URL('overview.png', dir), await page.screenshot());
    });
  }
}
