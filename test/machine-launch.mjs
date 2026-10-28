import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';

for (const engine of ['chromium', 'webkit']) {
  for (const { phone, game } of [{phone:false,game:'rally'},{phone:true,game:'rally'},{phone:false,game:'brawler'},{phone:true,game:'brawler'}]) {
    test(`${game} enters its real game geometry continuously in ${engine}, ${phone ? 'phone' : 'desktop'}`, async t => {
      const { page, origin } = await openPage(t, engine, {
        viewport: phone ? { width: 390, height: 844 } : { width: 1280, height: 800 }, hasTouch: phone,
      });
      // Hold the game so its first render cannot hide a broken document handoff.
      await page.route('**/*.wasm', () => {});
      await page.addInitScript(() => {
        addEventListener('pagereveal', event => {
          if (!event.viewTransition) return;
          window.__machineReveal = { title:!!document.getElementById('marquee-bar'), state:document.readyState,
            flags:[...document.documentElement.attributes].map(a=>[a.name,a.value]) };
          event.viewTransition.ready.then(() => {
            window.__machineAllAnimations = document.getAnimations().map(a => ({ name:a.animationName, pseudo:a.effect?.pseudoElement,
              target:a.effect?.target?.type, tag:a.effect?.target?.tagName }));
            window.__machineTransition = document.getAnimations().filter(a =>
              /view-transition-group\(machine-/.test(a.effect?.pseudoElement || '')).map(a => ({
              pseudo: a.effect.pseudoElement, duration: a.effect.getTiming().duration,
              frames: a.effect.getKeyframes(),
            }));
          }).catch(error => { window.__machineTransitionError = error.message; });
        });
      });
      await page.goto(`${origin}/index.html`);
      if(game!=='rally')await page.evaluate(game=>focusArcadeCard(document.querySelector(`.card[data-machine="${game}"]`)),game);
      await page.waitForFunction(game=>document.querySelector('.card-focused[data-loop-ready]')?.dataset.machine===game&&
        !document.getElementById('game-grid').hasAttribute('data-aisle-moving'),game);
      await page.waitForSelector('.card-focused[data-loop-ready]');
      await page.waitForFunction(() => !document.documentElement.hasAttribute('data-cabinet-arranging'));
      // A stale geometry cache from a different machine must not determine the destination.
      await page.evaluate(() => localStorage.setItem('blip-screen-box', JSON.stringify({
        w: innerWidth, h: innerHeight, box: [0, 0, 20, 20], deck: 500,
      })));
      const start = await page.locator('.card-focused .card-shot-wrap').boundingBox();
      await page.locator('.card-focused .machine-start').click();
      await page.waitForURL(`${origin}/${game}/index.html`);
      await page.waitForFunction(() => window.__machineTransition || window.__machineTransitionError);
        const result = await page.evaluate(() => ({ groups: window.__machineTransition, all:window.__machineAllAnimations, reveal:window.__machineReveal, error: window.__machineTransitionError,
        art: document.getElementById('launch-frame')?.style.backgroundImage,
        visible: getComputedStyle(document.body).visibility,
        target: document.getElementById('arcade-bezel').getBoundingClientRect().toJSON(),
        loader: getComputedStyle(document.getElementById('loader')).visibility }));
      assert.equal(result.error, undefined);
      for (const part of ['monitor', 'picture', 'title', 'name', 'deck']) {
        const group = result.groups.find(g => g.pseudo === `::view-transition-group(machine-${part})`);
        assert.ok(group, `${part} has a native geometry transition: ${JSON.stringify({reveal:result.reveal,all:result.all})}`);
        assert.equal(group.duration, 900);
        assert.ok(group.frames.length >= 2, `${part} has start and destination keyframes`);
      }
      assert.match(result.art, /data:image\/png;base64,/, 'the same frozen attract frame survives navigation');
      assert.equal(result.visible, 'visible');
      assert.equal(result.loader, 'hidden', 'no loading flash over the cabinet picture');
      assert.ok(result.target.width > start.width && result.target.width > 200, 'uses the real game screen, despite the stale cache');
      const monitorEnd = result.groups.find(g => g.pseudo === '::view-transition-group(machine-monitor)').frames.at(-1);
      assert.ok(Math.abs(parseFloat(monitorEnd.width) - result.target.width) < 1 &&
        Math.abs(parseFloat(monitorEnd.height) - result.target.height) < 1,
      `the screen stays at the transition endpoint after the shell finishes layout: ${JSON.stringify({end:monitorEnd,target:result.target,reveal:result.reveal})}`);
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
    await page.waitForURL(`${origin}/rally/index.html`);
    assert.equal(await page.locator('#launch-frame').count(),0);
    assert.equal(await page.evaluate(()=>document.documentElement.hasAttribute('data-machine-handoff')),false);
  });
}
