import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

const ALL_GAMES = ['ADDER', 'BOUNCER', 'BRAWLER', 'BUBBLER', 'DEFENDER', 'METEORS', 'RALLY', 'SERPENT', 'RAIDER'];
const GAMES = process.env.BLIP_CONTROL_GAME ? ALL_GAMES.filter((game) => game === process.env.BLIP_CONTROL_GAME) : ALL_GAMES;
const WASD_DECK = new Set(['ADDER', 'BRAWLER', 'BUBBLER']);

async function probe(t, touch) {
  const handle = await openPage(t, 'chromium', {
    hasTouch: touch, viewport: touch ? { width: 390, height: 844 } : { width: 1000, height: 900 },
  });
  const { page } = handle;
  // Observe the interpreter's actual key reads through its existing SOUND host call.
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){window.__controlMask=f-1000;window.__controlSamples=(window.__controlSamples||0)+1;playBasicTone(f,s,v);};');
    await route.fulfill({ response, body });
  });
  await page.goto(`${handle.origin}/basic.html`);
  const frame = () => page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
  return {
    ...handle,
    async load(game) {
      await page.evaluate((game) => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit(`LOAD ${game}`); }, game);
      await page.waitForFunction((game) => document.querySelector('#basic-output').textContent.includes(`LOADED /TAPE/${game}.BLIP`), game);
      const keys = WASD_DECK.has(game) ? ['A', 'D', 'W', 'S'] : ['LEFT', 'RIGHT', 'UP', 'DOWN'];
      keys.push(game === 'BUBBLER' || game === 'BRAWLER' ? 'F' : 'SPACE');
      keys.push(game === 'BUBBLER' || game === 'BRAWLER' ? 'G' : game === 'METEORS' ? 'Z' : 'SPACE');
      if (game === 'ADDER' || game === 'BUBBLER') keys.push('LEFT', 'RIGHT', 'J', game === 'ADDER' ? 'J' : 'K', 'UP', 'DOWN');
      await page.evaluate((keys) => {
        BlipBasic.lines.clear();
        BlipBasic.submit('10 CLEAR("BLIP_BLACK")');
        BlipBasic.submit(`20 SOUND 1000+${keys.map((key, i) => `${2 ** i}*KEY_HELD("${key}")`).join('+')},0,0`);
        BlipBasic.submit('30 WAIT 1:GOTO 20');
        BlipBasic.submit('RUN');
      }, keys);
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
      await this.mask(0);
      return keys;
    },
    async mask(expected) {
      const sampled = await frame().evaluate(() => window.__controlSamples || 0);
      try {
        await frame().waitForFunction(([mask, sampled]) => window.__controlSamples >= sampled + 3 && window.__controlMask === mask,
          [expected, sampled], { timeout: 3000 });
      } catch {
        assert.fail(`Expected key mask ${expected}, got ${await frame().evaluate(() => window.__controlMask)}; runtime error: ${await page.locator('body').getAttribute('data-wasm-error')}`);
      }
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    },
    async exit() {
      await page.locator('#exit-game').click();
      await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-playing'));
    },
  };
}

test('every BASIC game receives held and released physical PC keys', async (t) => {
  const p = await probe(t, false);
  for (const game of GAMES) await t.test(game, async (subtest) => {
    subtest.after(() => p.exit());
    const keys = await p.load(game);
    const canvas = p.page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    await canvas.focus();
    for (const [i, key] of keys.entries()) {
      if (keys.indexOf(key) !== i) continue;
      const physical = { LEFT: 'ArrowLeft', RIGHT: 'ArrowRight', UP: 'ArrowUp', DOWN: 'ArrowDown', SPACE: 'Space' }[key] || key.toLowerCase();
      const expected = keys.reduce((mask, other, index) => mask + (other === key ? 2 ** index : 0), 0);
      await p.page.keyboard.down(physical);
      await p.mask(expected);
      await p.page.keyboard.up(physical);
      await p.mask(0);
    }
  });
});

test('BASIC touch movement and buttons remain independent for every deck', async (t) => {
  const p = await probe(t, true);
  const cdp = await p.page.context().newCDPSession(p.page);
  let fingers = [];
  async function touch(type, id, point) {
    const lifted = fingers.find((finger) => finger.id === id);
    fingers = fingers.filter((finger) => finger.id !== id);
    if (type !== 'touchEnd') fingers.push({ id, ...point });
    await cdp.send('Input.dispatchTouchEvent', { type, touchPoints: type === 'touchEnd' ? [lifted] : fingers });
  }
  for (const game of GAMES.filter((game) => game !== 'RALLY')) await t.test(game, async (subtest) => {
    subtest.after(() => p.exit());
    await p.load(game);
    const box = await p.page.locator('#snes-pad .snes-dpad').boundingBox();
    const buttons = await p.page.locator('#snes-pad .snes-btn').evaluateAll((buttons) => buttons.map((button) => {
      const r = button.getBoundingClientRect();
      return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
    }));
    const point = { x: box.x + box.width * .9, y: box.y + box.height / 2 };
    const primary = game === 'BUBBLER' || game === 'BRAWLER' || game === 'METEORS' ? 16 : 48;
    const alternate = game === 'BUBBLER' || game === 'BRAWLER' || game === 'METEORS' ? 32 : 48;
    await touch('touchStart', 1, point); await p.mask(2);
    await touch('touchStart', 2, buttons[0]); await p.mask(2 + primary);
    await touch('touchEnd', 1); await p.mask(primary);
    await touch('touchEnd', 2); await p.mask(0);
    await touch('touchStart', 1, point); await p.mask(2);
    await touch('touchStart', 2, buttons[1]); await p.mask(2 + alternate);
    await touch('touchEnd', 2); await p.mask(2);
    await touch('touchEnd', 1); await p.mask(0);
    await touch('touchStart', 1, buttons[0]); await p.mask(primary);
    await touch('touchStart', 2, buttons[1]); await p.mask(primary | alternate);
    await touch('touchEnd', 1); await p.mask(alternate);
    await touch('touchEnd', 2); await p.mask(0);
  });
});
