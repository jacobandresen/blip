import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';
import { numberedTape } from './lib/basic-tape.mjs';
import { instrumentBasicSound } from './lib/basic-runtime.mjs';

const source = await readFile(new URL('../web/TAPE/ADDER.BLIP', import.meta.url), 'utf8');
function scene() {
  const lines = numberedTape(source);
  lines.set(85, 'START_GAME 1');
  for (const n of lines.keys()) if (n > 2710 && n < 2820) lines.delete(n);
  lines.set(2720, 'HEAD_X(S)=140+(S-1)*200:HEAD_Y(S)=220+(S-1)*160:ANGLE(S)=(S-1)*3.141592653589793');
  lines.set(2730, 'IF S>=3 THEN HEAD_X(S)=100+(S-3)*80:HEAD_Y(S)=100:ANGLE(S)=0');
  return lines;
}

async function open(t, touch) {
  const handle = await openPage(t, 'chromium', { hasTouch: touch, viewport: touch ? { width: 390, height: 844 } : { width: 1000, height: 900 } });
  await instrumentBasicSound(handle.page, 'function(f,s,v){window.__adderFlags=(window.__adderFlags||0)|(f-1000);}');
  await handle.page.emulateMedia({ reducedMotion: 'reduce' });
  await handle.page.goto(`${handle.origin}/basic.html`);
  await handle.page.evaluate(() => BlipBasic.submit('LOAD /TAPE/ADDER'));
  return {
    ...handle,
    frame: () => handle.page.frames().find((frame) => frame.url().includes('basic-runtime.html')),
    async start(lines) {
      await handle.page.evaluate((lines) => {
        BlipBasic.lines.clear();
        for (const [n, code] of lines) BlipBasic.lines.set(n, code);
        BlipBasic.submit('SAVE MY_VIPERS');
        BlipBasic.submit('RUN');
      }, [...lines]);
      await handle.page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
      assert.equal(await handle.page.locator('body').getAttribute('data-wasm-error'), null);
      await handle.page.frameLocator('#basic-wasm-frame').locator('#glcanvas').focus();
    },
    async flags(bits) {
      try { await this.frame().waitForFunction((bits) => (window.__adderFlags & bits) === bits, bits, { timeout: 7000 }); }
      catch { assert.fail(`Missing Adder flags ${bits}; got ${await this.frame().evaluate(() => window.__adderFlags)}; ${await handle.page.locator('body').getAttribute('data-wasm-error')}`); }
    },
  };
}

for (const touch of [false, true]) test(`Adder BASIC keeps continuous steering and player-two joining on ${touch ? 'touch' : 'PC'}`, async (t) => {
  const p = await open(t, touch);
  const lines = scene();
  lines.set(165, 'SOUND 1000+(ANGLE(1)<-0.15)+2*(ANGLE(1)>0.15)+4*HUMAN(2)+8*(ANGLE(2)>3.3)+16*(ANGLE(2)<2.98)+32*(KEY_HELD("A") AND KEY_HELD("D") AND TURN(1)=0),0,0');
  await p.start(lines);
  const cdp = touch ? await p.page.context().newCDPSession(p.page) : null;
  async function down(key, selector, side = .5) {
    await p.frame().evaluate(() => { window.__adderFlags = 0; });
    if (!touch) return p.page.keyboard.down(key);
    const box = await p.page.locator(selector).boundingBox();
    assert.ok(box && box.width > 0, selector);
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ id: 1, x: box.x + box.width * side, y: box.y + box.height / 2 }] });
  }
  async function up(key) {
    if (!touch) return p.page.keyboard.up(key);
    await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  }
  await down('a', '#snes-pad .snes-dpad', .1); await p.flags(1); await up('a');
  await down('d', '#snes-pad .snes-dpad', .9); await p.flags(2); await up('d');
  await down('j', '#snes-pad-p2 .snes-btn.cap1'); await p.flags(4); await up('j');
  await down('ArrowLeft', '#snes-pad-p2 .snes-dpad', .1); await p.flags(16); await up('ArrowLeft');
  await down('ArrowRight', '#snes-pad-p2 .snes-dpad', .9); await p.flags(8); await up('ArrowRight');
  if (!touch) {
    await p.page.keyboard.down('a'); await p.page.keyboard.down('d');
    await p.flags(32);
    await p.page.keyboard.up('a'); await p.page.keyboard.up('d');
  }
  assert.equal(await p.page.locator('body').getAttribute('data-wasm-error'), null);
});

test('Adder BASIC carries lives and scores into the next round and adds a CPU', async (t) => {
  const p = await open(t, false);
  const lines = scene();
  lines.set(2740, 'IF S=1 AND ROUND=1 THEN HEAD_X(1)=627:ANGLE(1)=0');
  lines.set(165, 'SOUND 1000+63*(ROUND=2 AND LIFE(1)=2 AND SEATS=3 AND LIFE(2)=1 AND PTS(2)=250 AND ALIVE(1)=1),0,0');
  await p.start(lines);
  await p.flags(63);
  assert.equal(await p.page.locator('body').getAttribute('data-wasm-error'), null);
});

test('Adder BASIC can start with two humans and a CPU from the title', async (t) => {
  const p = await open(t, false);
  const lines = scene();
  lines.set(85, 'START_DEMO');
  lines.set(165, 'SOUND 1000+(STATE$="PLAY" AND HUMAN(1) AND HUMAN(2) AND LIFE(2)=3 AND SEATS=3),0,0');
  await p.start(lines);
  await p.page.keyboard.press('j', { delay: 80 });
  await p.flags(1);
});
