import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const engine of ['webkit', 'chromium']) {
  test(`touchscreen control selection uses gesture strips, preserving Rally, in ${engine}`, async (t) => {
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
      const choice = state.query === '?controls=stick' ? 'stick' : state.query === '?controls=pad' ? 'pad' : 'touch';
      assert.equal(state.selected, choice, JSON.stringify(state));
      assert.equal(state.touch, choice === 'touch' ? state.kind : null, JSON.stringify(state));
      assert.equal(state.effective, choice, JSON.stringify(state));
    }

    await page.goto(`${origin}/meteors/index.html`);
    await page.waitForSelector('#touch-strip');
    assert.equal(await page.locator('#touch-strip').isVisible(), true, 'action games use touch controls');
    assert.equal(await page.locator('.ts-action').count(), 2, 'fire and hyperspace have touch zones');
    assert.equal(await page.locator('#snes-pad').isVisible(), false, 'the pad is hidden');
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

for (const engine of ['webkit', 'chromium']) test(`a touch player can choose joystick or D-pad and keep that choice in ${engine}`, async (t) => {
  const { page, origin } = await openPage(t, engine, { hasTouch: true, viewport: { width: 810, height: 1080 } });
  await page.goto(`${origin}/meteors/index.html`);
  assert.equal(await page.evaluate(() => blipControls()), 'touch');
  await page.locator('#card-button').click();
  for (const [choice, selector] of [['pad', '#snes-pad'], ['stick', '#stick-base'], ['touch', '#touch-strip'], ['pad', '#snes-pad']]) {
    await page.locator(`[data-control-choice="${choice}"]`).click();
    assert.equal(await page.evaluate(() => blipControls()), choice);
    assert.equal(await page.locator(selector).isVisible(), true, `${choice} is visible`);
    assert.equal(await page.locator('#controls-card').isVisible(), true, 'the card stays open while choosing');
    if (choice !== 'touch') assert.match(await page.locator('#controls-card .pc-legend').textContent(), /RED BUTTON/);
  }
  await page.reload();
  assert.equal(await page.evaluate(() => blipControls()), 'pad', 'the choice survives a new page load');
  await page.goto(`${origin}/raider/index.html`);
  assert.equal(await page.evaluate(() => blipControls()), 'pad', 'the choice applies to the next game');
  await page.goto(`${origin}/bouncer/index.html?controls=stick`);
  assert.equal(await page.evaluate(() => blipControls()), 'stick', 'an explicit link takes precedence');
  await page.locator('#card-button').click();
  await page.locator('[data-control-choice="touch"]').click();
  assert.equal(new URL(page.url()).searchParams.has('controls'), false, 'a new choice clears the link override');
  await page.reload();
  assert.equal(await page.evaluate(() => blipControls()), 'touch');
});

test('action touch zones steer and hold game buttons without a pad', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 810, height: 1080 } });
  for (const [slug, actionKey] of [['meteors', ' '], ['brawler', 'f'], ['raider', ' ']]) {
    await page.goto(`${origin}/${slug}/index.html`);
    await page.evaluate(() => document.getElementById('insert-coin-btn').click());
    const events = await page.evaluate(() => {
      const canvas = document.querySelector('canvas');
      const steer = document.querySelector('.ts-half');
      const button = document.querySelector('.ts-action');
      const got = [];
      canvas.addEventListener('keydown', (e) => got.push(['down', e.key]));
      canvas.addEventListener('keyup', (e) => got.push(['up', e.key]));
      const send = (target, type, id, x, y) => target.dispatchEvent(new PointerEvent(type, {
        bubbles: true, cancelable: true, pointerId: id, pointerType: 'mouse', clientX: x, clientY: y,
      }));
      send(steer, 'pointerdown', 1, 100, 100);
      send(steer, 'pointermove', 1, 150, 70);
      send(button, 'pointerdown', 2, 700, 900);
      send(button, 'pointerup', 2, 700, 900);
      send(steer, 'pointerup', 1, 150, 70);
      return got;
    });
    assert.ok(events.some(([type, key]) => type === 'down' && (key === 'ArrowRight' || key === 'd')), `${slug} steers right`);
    assert.ok(events.some(([type, key]) => type === 'up' && (key === 'ArrowRight' || key === 'd')), `${slug} releases steering`);
    assert.ok(events.some(([type, key]) => type === 'down' && key === actionKey), `${slug} presses its action`);
    assert.ok(events.some(([type, key]) => type === 'up' && key === actionKey), `${slug} releases its action`);
  }
});

test('Brawler gives both touch players independent movement and attacks', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 810, height: 1080 } });
  await page.goto(`${origin}/brawler/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.evaluate(() => document.getElementById('insert-coin-btn').click());
  const result = await page.evaluate(() => {
    blipSetMode(1);
    const canvas = document.querySelector('canvas');
    const glass = document.querySelector('.ts-glass').getBoundingClientRect();
    const steer = document.querySelector('.ts-half[data-slot="1"]');
    const kick = document.querySelector('.ts-actions[data-player="1"] .ts-action[data-button="2"]');
    const keys = [];
    canvas.addEventListener('keydown', (e) => keys.push(['down', e.key]));
    canvas.addEventListener('keyup', (e) => keys.push(['up', e.key]));
    const send = (target, type, id, x) => target.dispatchEvent(new PointerEvent(type, {
      bubbles: true, cancelable: true, pointerId: id, pointerType: 'mouse', clientX: x, clientY: glass.top + 30,
    }));
    const x = glass.left + glass.width * .62;
    send(steer, 'pointerdown', 1, x);
    send(steer, 'pointermove', 1, x + 40);
    send(kick, 'pointerdown', 2, glass.right - 25);
    send(kick, 'pointerup', 2, glass.right - 25);
    send(steer, 'pointerup', 1, x + 40);
    return { keys, visible: getComputedStyle(kick).display !== 'none' };
  });
  assert.equal(result.visible, true);
  for (const [type, key] of [['down', 'ArrowRight'], ['up', 'ArrowRight'], ['down', 'k'], ['up', 'k']]) {
    assert.ok(result.keys.some((event) => event[0] === type && event[1] === key), `${type} ${key}`);
  }
});

test('two real touch contacts keep Brawler steering and attacking together', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 810, height: 1080 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 810, height: 1080, deviceScaleFactor: 2, mobile: true });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });
  await page.goto(`${origin}/brawler/index.html`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'));
  await page.evaluate(() => {
    document.getElementById('insert-coin-btn').click();
    blipSetMode(1);
    window.__touchKeys = [];
    const canvas = document.querySelector('canvas');
    canvas.addEventListener('keydown', (e) => window.__touchKeys.push(['down', e.key]));
    canvas.addEventListener('keyup', (e) => window.__touchKeys.push(['up', e.key]));
  });
  const glass = await page.locator('.ts-glass').boundingBox();
  const kick = await page.locator('.ts-actions[data-player="1"] .ts-action[data-button="2"]').boundingBox();
  const steer = { id: 1, x: glass.x + glass.width * .61, y: glass.y + glass.height * .45 };
  const attack = { id: 2, x: kick.x + kick.width / 2, y: kick.y + kick.height / 2 };
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [steer] });
  steer.x += 45;
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [steer] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [steer, attack] });
  const held = await page.evaluate(() => window.__touchKeys);
  assert.ok(held.some(([type, key]) => type === 'down' && key === 'ArrowRight'));
  assert.ok(held.some(([type, key]) => type === 'down' && key === 'k'));
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const released = await page.evaluate(() => window.__touchKeys);
  assert.ok(released.some(([type, key]) => type === 'up' && key === 'ArrowRight'));
  assert.ok(released.some(([type, key]) => type === 'up' && key === 'k'));
});

test('changing controller style releases a held touch direction', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 810, height: 1080 } });
  await page.goto(`${origin}/raider/index.html`);
  await page.evaluate(() => document.getElementById('insert-coin-btn').click());
  const result = await page.evaluate(() => {
    const canvas = document.querySelector('canvas'), steer = document.querySelector('.ts-half');
    const events = [];
    canvas.addEventListener('keydown', (e) => events.push(['down', e.key]));
    canvas.addEventListener('keyup', (e) => events.push(['up', e.key]));
    const send = (type, x) => steer.dispatchEvent(new PointerEvent(type, {
      bubbles: true, cancelable: true, pointerId: 7, pointerType: 'mouse', clientX: x, clientY: 100,
    }));
    send('pointerdown', 100);
    send('pointermove', 150);
    blipSetControls('pad');
    return { events, controls: blipControls() };
  });
  assert.equal(result.controls, 'pad');
  assert.ok(result.events.some(([type, key]) => type === 'down' && key === 'ArrowRight'));
  assert.ok(result.events.some(([type, key]) => type === 'up' && key === 'ArrowRight'));
});

test('control choices and card fit an iPhone landscape safe area', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 844, height: 390 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true });
  await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets: { top: 0, bottom: 21, left: 47, right: 47 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const slug of ['meteors', 'brawler']) {
    await page.goto(`${origin}/${slug}/index.html`);
    await page.locator('#card-button').click();
    for (const [choice, selector] of [['touch', '.ts-glass'], ['stick', '#stick-base'], ['pad', '#snes-pad .snes-dpad']]) {
      await page.locator(`[data-control-choice="${choice}"]`).click();
      const box = await page.locator(selector).boundingBox();
      const card = await page.locator('#controls-card').boundingBox();
      assert.ok(box && box.x >= 46 && box.x + box.width <= 798 && box.y + box.height <= 371, `${slug} ${choice} fits: ${JSON.stringify(box)}`);
      assert.ok(card && card.x >= 46 && card.x + card.width <= 798 && card.y + card.height <= 391, `${slug} card fits: ${JSON.stringify(card)}`);
    }
  }
});
