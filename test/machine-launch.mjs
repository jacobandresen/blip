import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage, arcadeArranged, arrivedAt } from './lib/harness.mjs';

for (const engine of ['chromium']) {
  for (const { phone, game } of [{phone:false,game:'rally'},{phone:true,game:'rally'},{phone:false,game:'brawler'},{phone:true,game:'brawler'}]) {
    // The player walks up to the machine until its monitor stands where the game page's will (arcade.js arcadeWalk), and
    // the pages cross-fade.
    test(`${game} walks up to its real game geometry in ${engine}, ${phone ? 'phone' : 'desktop'}`, async t => {
      const { page, origin } = await openPage(t, engine, {
        viewport: phone ? { width: 390, height: 844 } : { width: 1280, height: 800 }, hasTouch: phone,
      });
      // Hold the game so its first render cannot hide a broken document handoff.
      await page.route('**/*.wasm', () => {});
      await page.addInitScript(() => {
        // Where the arcade's monitor stood as the walk ended and the page left.
        addEventListener('pageswap', () => {
          const r = document.querySelector('#game-grid .card-launching .card-shot-wrap')?.getBoundingClientRect();
          if (r) sessionStorage.setItem('test-machine-from', JSON.stringify({ x: r.left, y: r.top, w: r.width, h: r.height }));
        });
        addEventListener('pagereveal', event => {
          if (!event.viewTransition) return;
          window.__machineFrom = JSON.parse(sessionStorage.getItem('test-machine-from') || 'null');
          event.viewTransition.ready.then(() => {
            window.__machineTransition = document.getAnimations().filter(a => /view-transition-(old|new)\(root\)/.test(a.effect?.pseudoElement || ''))
              .map(a => ({ pseudo: a.effect.pseudoElement, duration: a.effect.getTiming().duration }));
          }).catch(error => { window.__machineTransitionError = error.message; });
        });
      });
      await page.goto(`${origin}/index.html`);
      if(game!=='rally')await page.evaluate(game=>focusArcadeCard(document.querySelector(`.card[data-machine="${game}"]`)),game);
      await page.waitForFunction(game=>document.querySelector('.card-focused[data-loop-ready]')?.dataset.machine===game&&
        !document.getElementById('game-grid').hasAttribute('data-aisle-moving'),game);
      await page.waitForSelector('.card-focused[data-loop-ready]');
      await arcadeArranged(page);
      // A stale geometry cache from a different machine must not determine the destination.
      await page.evaluate(() => localStorage.setItem('blip-screen-box', JSON.stringify({
        w: innerWidth, h: innerHeight, box: [0, 0, 20, 20], deck: 500,
      })));
      const start = await page.locator('.card-focused .card-shot-wrap').boundingBox();
      await page.locator('.card-focused .machine-start').click();
      await arrivedAt(page, `${origin}/${game}/index.html`);
      await page.waitForFunction(() => window.__machineTransition || window.__machineTransitionError);
        const result = await page.evaluate(() => ({ groups: window.__machineTransition, from: window.__machineFrom, error: window.__machineTransitionError,
        art: document.getElementById('launch-frame')?.style.backgroundImage,
        visible: getComputedStyle(document.body).visibility,
        target: document.getElementById('arcade-bezel').getBoundingClientRect().toJSON(),
        loader: getComputedStyle(document.getElementById('loader')).visibility }));
      assert.equal(result.error, undefined);
      assert.ok(result.groups.length >= 2, 'the pages cross-fade');
      assert.match(result.art, /data:image\/png;base64,/, 'the same frozen attract frame survives navigation');
      assert.equal(result.visible, 'visible');
      assert.equal(result.loader, 'hidden', 'no loading flash over the cabinet picture');
      assert.ok(result.target.width > start.width && result.target.width > 200, 'uses the real game screen, despite the stale cache');
      // The walk ends on the game page's monitor: from the layout the page will have at this size, not the stale cache.
      const from = result.from, target = result.target;
      t.diagnostic(JSON.stringify({ from, target }));
      assert.ok(from, 'the arcade said where its monitor stood');
      const grow = from.w / target.width, off = Math.hypot(from.x + from.w / 2 - target.x - target.width / 2, from.y + from.h / 2 - target.y - target.height / 2);
      // On a computer the cabinet's monitor is the page's shape, so it lands on it; a phone's page is taller.
      const close = phone ? { grow: .03, off: target.height * .4 } : { grow: .01, off: 4 };
      assert.ok(Math.abs(grow - 1) < close.grow && off < close.off, `the walk ends on the game page's monitor: ${JSON.stringify({ from, target })}`);
      const dir = new URL(`artifacts/machine-launch/${engine}-${phone ? 'phone' : 'desktop'}-${game}/`, import.meta.url);
      await mkdir(dir, { recursive: true });
      await writeFile(new URL('geometry.json', dir), JSON.stringify({ start, ...result }, null, 2));
      await page.waitForFunction(() => !document.documentElement.hasAttribute('data-machine-handoff'));
      await page.screenshot({ path: new URL('arrived.png', dir).pathname });
    });
  }
  test(`reduced motion enters the game directly in ${engine}`,async t=>{
    const {page,origin}=await openPage(t,engine,{viewport:{width:390,height:844},hasTouch:true});
    await page.emulateMedia({reducedMotion:'reduce'});
    await page.route('**/*.wasm',()=>{});
    await page.goto(`${origin}/index.html`);
    await page.waitForSelector('.card-focused .machine-start');
    await page.locator('.card-focused .machine-start').click();
    await arrivedAt(page, `${origin}/rally/index.html`);
    assert.equal(await page.locator('#launch-frame').count(),0);
    assert.equal(await page.evaluate(()=>document.documentElement.hasAttribute('data-machine-handoff')),false);
  });
}
