import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { openPage } from './lib/harness.mjs';

for (const touch of [false, true]) test(`Bubbler BASIC players move, jump, join, and blow bubbles with ${touch ? 'touch' : 'PC keys'}`, async (t) => {
  const { page, origin } = await openPage(t, 'chromium', {
    hasTouch: touch, viewport: touch ? { width: 390, height: 844 } : { width: 1000, height: 900 },
  });
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){window.__gameFlags=(window.__gameFlags||0)|(f-1000);};');
    await route.fulfill({ response, body });
  });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('LOAD /TAPE/BUBBLER.BLIP'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BUBBLER.BLIP'));
  // Keep the real player updates; isolate them from the toys and the end of the round.
  const roundWon = readFileSync(new URL('../web/TAPE/BUBBLER.BLIP', import.meta.url), 'utf8')
    .split('\n').find((line) => line.replace(/^\d+\s+/, '').trim() === 'ROUND_WON').match(/^\d+/)[0];
  await page.evaluate((roundWon) => {
    BlipBasic.lines.set(80, 'REM NO MUSIC');
    BlipBasic.lines.set(85, 'START_GAME:SHOW "PLAY":FOR I=1 TO TOY_COUNT:TOY_ON(I)=0:NEXT I');
    BlipBasic.lines.set(Number(roundWon), 'REM THE ROUND NEVER ENDS');
    BlipBasic.lines.set(165, 'SOUND 1000+TWO+2*(PX(1)>SPAWN_X(1)+10)+4*(PY(1)<SPAWN_Y(1)-10)+8*(PX(2)<SPAWN_X(2)-10)+16*(PY(2)<SPAWN_Y(2)-10)+32*(BLOW_WAIT(1)>0)+64*(BLOW_WAIT(2)>0),0,0');
    BlipBasic.submit('SAVE MY_BUBBLES');
    BlipBasic.submit('RUN');
  }, roundWon);
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const frame = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  async function observed(bits) {
    try {
      await frame.waitForFunction((bits) => (window.__gameFlags & bits) === bits, bits, { timeout: 4000 });
    } catch {
      const controls = await page.evaluate(() => Array.from(document.querySelectorAll('.amstrad-controls .stick-base,.amstrad-controls .arcade-btn')).map((el) => ({ id: el.id, player: el.closest('.deck-station')?.id, rect: el.getBoundingClientRect().toJSON(), pressed: el.classList.contains('blip-pressed') })));
      assert.fail(`Missing gameplay flags ${bits}; observed ${await frame.evaluate(() => window.__gameFlags)}; ${await page.locator('body').getAttribute('data-wasm-error')}; ${JSON.stringify(controls)}`);
    }
  }
  async function fresh() { await frame.evaluate(() => { window.__gameFlags = 0; }); }
  const cdp = touch ? await page.context().newCDPSession(page) : null;
  async function down(key, selector, side = .5) {
    await fresh();
    if (!touch) return page.keyboard.down(key);
    const box = await page.locator(selector).boundingBox();
    assert.ok(box && box.width > 0 && box.height > 0, `${selector} must be visible`);
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: box.x + box.width * side, y: box.y + box.height / 2 }] });
  }
  async function up(key) {
    if (!touch) return page.keyboard.up(key);
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  }
  await down('d', '#stick-base', .9); await observed(2); await up('d');
  await down('g', '#fire-buttons .arcade-btn:nth-child(2)'); await observed(4); await up('g');
  await down('f', '#fire-buttons .arcade-btn:first-child'); await observed(32); await up('f');
  await down('j', '#deck-p2 .fire-buttons .arcade-btn:first-child'); await observed(1 | 64); await up('j');
  await down('ArrowLeft', '#deck-p2 .stick-base', .1); await observed(8); await up('ArrowLeft');
  await down('k', '#deck-p2 .fire-buttons .arcade-btn:nth-child(2)'); await observed(16); await up('k');
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
});

test('Rally BASIC touch dials move both real paddles independently', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 } });
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){window.__paddleFlags=(window.__paddleFlags||0)|(f-1000);};');
    await route.fulfill({ response, body });
  });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('LOAD /TAPE/RALLY.BLIP'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/RALLY.BLIP'));
  await page.evaluate(() => {
    BlipBasic.lines.set(80, 'PLAYERS=2:START_GAME');
    BlipBasic.lines.set(175, 'SOUND 1000+(LPAD_Y>HOME_Y)+2*(RPAD_Y>HOME_Y),0,0');
    BlipBasic.submit('SAVE MY_PADDLES');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const frame = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
  const cdp = await page.context().newCDPSession(page);
  for (const [player, bits] of [[1, 1], [2, 3]]) {
    const box = await page.locator(`#amstrad-rally-dial-p${player}`).boundingBox();
    const x = box.x + box.width / 2, y = box.y + box.height / 2, r = box.width * .3;
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: x + r, y }] });
    for (let step = 1; step <= 5; step++) {
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ id: 1, x: x + Math.cos(step * .25) * r, y: y + Math.sin(step * .25) * r }] });
      await page.waitForTimeout(25);
    }
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    await frame.waitForFunction((bits) => window.__paddleFlags === bits, bits, { timeout: 3000 });
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
});
