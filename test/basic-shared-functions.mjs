import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('the console loads Rust math lazily and preserves queued command order', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  assert.equal(await page.locator('#basic-wasm-frame').getAttribute('src'), null);
  await page.evaluate(async () => {
    const first = BlipBasic.submit('X=DISTANCE(-2,-3,1,1)');
    BlipBasic.submit('X=X+CLAMP(8,-2,3)');
    BlipBasic.submit('PRINT "SHARED RESULT",X');
    await first;
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('SHARED RESULT 8'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-playing'), null);
  assert.equal(await page.locator('#basic-input').evaluate((input) => document.activeElement === input), true);
  const values = await page.evaluate(() => {
    const exports = document.querySelector('#basic-wasm-frame').contentWindow.wasm_exports;
    return [exports.blip_basic_math_wrap(-21, 0, 10), exports.blip_basic_math_distance(-2, -3, 1, 1)];
  });
  assert.deepEqual(values, [9, 5]);
  await page.evaluate(() => BlipBasic.submit('PRINT "BOUNDS",MIN(-3,MAX(2,9)),ATAN2(1,-1)'));
  assert.match(await page.locator('#basic-output').textContent(), /BOUNDS -3 2\.35619449/);
  await page.evaluate(() => BlipBasic.submit('PRINT CLAMP(1)'));
  assert.match(await page.locator('#basic-output').textContent(), /CLAMP needs 3 arguments/i);
  await page.evaluate(() => BlipBasic.submit('PRINT DISTANCE("X",0,1,1)'));
  assert.match(await page.locator('#basic-output').textContent(), /DISTANCE needs numeric arguments/i);
});

test('text programs use shared Rust functions without opening the game monitor', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async () => {
    for (const line of ['10 A(1)=WRAP(-21,0,10)', '20 PRINT "TEXT RESULT",A(1)+DISTANCE(0,0,3,4)', '30 END']) BlipBasic.submit(line);
    await BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('TEXT RESULT 14'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-playing'), null);
});

test('Rust graphics programs share the native math and collision boundaries', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async () => {
    const source = [
      '10 CLEAR("BLIP_BLACK")',
      '20 IF DISTANCE(-2,-3,1,1)<>5 THEN DRAW_TEXTURE("BAD DISTANCE",0,0,1,1)',
      '30 IF RECTS_OVERLAP(0,0,2,2,2,0,1,1) THEN DRAW_TEXTURE("BAD EDGE",0,0,1,1)',
      '40 IF NOT SEGMENT_CIRCLE_OVERLAP(0,0,10,0,5,1,1) THEN DRAW_TEXTURE("BAD TANGENT",0,0,1,1)',
      '50 IF WRAP(-21,0,10)<>9 THEN DRAW_TEXTURE("BAD WRAP",0,0,1,1)',
      '60 SOUND 440,0.02,0.1',
      '70 WAIT 16:GOTO 70',
    ];
    for (const line of source) BlipBasic.submit(line);
    await BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-error') || document.querySelector('#basic-wasm-frame').contentWindow.blipBasicSoundEvents > 0);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
});
