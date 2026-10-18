import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

async function features(page, image) {
  return page.evaluate(async (encoded) => {
    const img = new Image(); img.src = `data:image/png;base64,${encoded}`; await img.decode();
    const canvas = document.createElement('canvas'); canvas.width = img.width; canvas.height = img.height;
    const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
    const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);
    let visible = 0, green = 0, purple = 0, cyan = 0, brown = 0;
    for (let i = 0; i < data.length; i += 4) {
      const [r, g, b] = data.slice(i, i + 3);
      if (r + g + b > 55) visible++;
      if (g > r * 1.08 && g > b * 1.15) green++;
      if (r > g * 1.1 && b > g * 1.1) purple++;
      if (g > r * 1.2 && b > r * 1.2 && Math.abs(g - b) < 90) cyan++;
      if (r > g * 1.12 && g > b * 1.15 && r < 130) brown++;
    }
    return { width, height, visible: visible / (width * height), green, purple, cyan, brown };
  }, image.toString('base64'));
}

async function runBasic(page, origin, trial) {
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit('LOAD ADDER'); });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/ADDER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  try {
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  } catch {
    const debug = await page.evaluate(() => ({ output: document.querySelector('#basic-output').textContent, sourceLength: document.querySelector('#basic-wasm-frame').contentWindow?.blipBasicSource?.length, frame: document.querySelector('#basic-wasm-frame').getAttribute('src') }));
    assert.fail(`BASIC trial ${trial} failed to start: ${JSON.stringify(debug)}`);
  }
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `BASIC trial ${trial} starts cleanly`);
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.keyboard.press('Space');
  await page.waitForTimeout(180);
  const before = await canvas.screenshot();
  await page.keyboard.down('ArrowLeft'); await page.waitForTimeout(120); await page.keyboard.up('ArrowLeft');
  await page.waitForTimeout(160);
  const after = await canvas.screenshot();
  assert.notDeepEqual(after, before, `BASIC trial ${trial} moves and turns in response to the arrows`);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, `BASIC trial ${trial} has no runtime error`);
  const result = await features(page, after);
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
  return result;
}

async function runRust(page, origin, trial) {
  await page.goto(`${origin}/adder/index.html?amstrad=1&playtest=${trial}`);
  await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), undefined, { timeout: 20000 });
  if (await page.locator('#need-coin-overlay').evaluate((el) => el.classList.contains('visible'))) {
    await page.locator('#insert-coin-btn').click();
    await page.waitForFunction(() => !document.querySelector('#need-coin-overlay').classList.contains('visible'));
  }
  const canvas = page.locator('#glcanvas');
  await canvas.focus(); await page.keyboard.press('Space'); await page.waitForTimeout(180);
  const before = await canvas.screenshot();
  await page.keyboard.down('ArrowLeft'); await page.waitForTimeout(120); await page.keyboard.up('ArrowLeft');
  await page.waitForTimeout(160);
  const after = await canvas.screenshot();
  assert.notDeepEqual(after, before, `Rust trial ${trial} moves and turns in response to the arrows`);
  return features(page, after);
}

test('five Adder playtests compare BASIC and Rust gameplay under the same arrow inputs', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/paint.js*', (route) => route.abort());
  const results = [];
  for (let trial = 1; trial <= 5; trial++) {
    const basic = await runBasic(page, origin, trial);
    const rust = await runRust(page, origin, trial);
    assert.ok(basic.visible > 0.005 && rust.visible > 0.005, `trial ${trial}: both games render a visible pit`);
    assert.ok(basic.green > 20 && rust.green > 20, `trial ${trial}: both games render the player viper (BASIC ${basic.green}, Rust ${rust.green})`);
    results.push({ trial, basic: `${basic.width}x${basic.height}`, rust: `${rust.width}x${rust.height}`, basicGreen: basic.green, rustGreen: rust.green, rustCpuColors: { purple: rust.purple, cyan: rust.cyan } });
  }
  console.log(`ADDER five playtests: ${JSON.stringify(results)}`);
});

test('Adder BASIC yields across long trail frames instead of stopping at the frame budget', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/paint.js*', (route) => route.abort());
  await page.goto(`${origin}/basic.html`);
  const source = await page.evaluate(async () => {
    const response = await fetch('/TAPE/ADDER.BLIP');
    const original = await response.text();
    return original.replace('GAME_STATE=1:SCORE=0:LIVES=3:ROUND=1:LENGTH=4:', 'GAME_STATE=1:SCORE=0:LIVES=3:ROUND=1:LENGTH=320:');
  });
  assert.ok(source.includes('GAME_STATE=1:SCORE=0:LIVES=3:ROUND=1:LENGTH=320:'), 'the stress fixture starts with a long trail');
  await page.evaluate((text) => {
    BlipBasic.filesystem.files['/USER/ADDER.BLIP'] = text.toUpperCase();
    BlipBasic.writeFilesystem();
    BlipBasic.submit('LOAD /USER/ADDER.BLIP');
  }, source);
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /USER/ADDER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
  await page.waitForTimeout(3000);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'long trail frames keep running without the old statement budget error');
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.keyboard.press('Space');
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null, 'the long trail scene remains interactive');
  await page.locator('#exit-game').click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-wasm-running'));
});
