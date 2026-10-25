import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const engine of ['webkit', 'chromium']) {
  test(`touchscreen control selection uses gesture strips or pads, preserving Rally, in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { hasTouch: true, viewport: { width: 390, height: 844 } });
    await page.goto(`${origin}/bouncer/index.html`);
    await page.waitForSelector('#touch-strip');
    assert.equal(await page.locator('#touch-strip').isVisible(), true, 'the arcade opens a touch strip');
    assert.equal(await page.locator('#stick-base').isVisible(), false, 'no arcade stick');
    assert.equal(await page.locator('#snes-pad').isVisible(), false, 'no duplicate pad behind the strip');

    // Exercise the shared first-paint policy over every game, front end and legacy override.
    const policies = await page.evaluate(() => {
      const results = [];
      for (const frontend of ['home', 'basic', 'cabinet']) {
        blipSetFrontend(frontend);
        for (const slug of Object.keys(BLIP_GAMES)) {
          for (const query of ['', '?controls=stick', '?controls=pad']) {
            history.replaceState(null, '', `/${slug}/index.html${query}`);
            blipApplyControls();
            results.push({ slug, query, frontend, selected: blipFrontendControls(), effective: blipControls(),
              touch: document.documentElement.getAttribute('data-touch'), kind: BLIP_GAMES[slug].touch?.kind || null });
          }
        }
      }
      return results;
    });
    for (const state of policies) {
      if (state.slug === 'rally') {
        if (state.query === '') assert.equal(state.selected, { cabinet: 'stick', home: 'touch', basic: 'pad' }[state.frontend], 'Rally retains its front-end selection');
        if (state.query === '?controls=pad') assert.equal(state.selected, 'pad');
        continue;
      }
      assert.equal(state.selected, 'touch', JSON.stringify(state));
      assert.equal(state.touch, state.kind, JSON.stringify(state));
      assert.equal(state.effective, state.kind ? 'touch' : 'pad', JSON.stringify(state));
    }

    await page.goto(`${origin}/meteors/index.html?controls=stick`);
    await page.waitForSelector('#snes-pad');
    assert.equal(await page.locator('#snes-pad').isVisible(), true, 'games without a strip use touch buttons');
    assert.equal(await page.locator('#stick-base').isVisible(), false);
    await page.goto(`${origin}/rally/index.html`);
    await page.waitForSelector('#paddle-dial');
    assert.equal(await page.locator('#paddle-dial').isVisible(), true, 'Rally retains its dials');
  });
}

test('a desktop keeps the arcade controls and explicit deck choices', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1280, height: 800 } });
  await page.goto(`${origin}/bouncer/index.html`);
  assert.equal(await page.evaluate(() => blipControls()), 'stick');
  await page.goto(`${origin}/bouncer/index.html?controls=pad`);
  assert.equal(await page.evaluate(() => blipControls()), 'pad');
});
