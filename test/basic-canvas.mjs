import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('BASIC switches title and play canvases without losing queued drawing or input', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    for (const line of [
      '10 CANVAS(480,540):CLEAR("BLIP_GREEN")',
      '20 IF KEY_PRESSED("SPACE") THEN CANVAS(320,200):GOTO 100',
      '30 WAIT 16:GOTO 10',
      '100 CLEAR("BLIP_RED")',
      '110 IF KEY_PRESSED("F") THEN CANVAS(624,600):GOTO 200',
      '120 WAIT 16:GOTO 100',
      '200 CLEAR("BLIP_BLUE")',
      '210 IF KEY_PRESSED("G") THEN CANVAS(480,540):GOTO 10',
      '220 WAIT 16:GOTO 200',
    ]) BlipBasic.submit(line);
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  const frame = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
  async function picture(width, height, color) {
    try {
      await frame.waitForFunction(([w, h]) => blipPicture.w === w && blipPicture.h === h, [width, height], { timeout: 3000 });
    } catch {
      assert.fail(`Expected ${width}x${height}; got ${JSON.stringify(await frame.evaluate(() => blipPicture))}; runtime error: ${await page.locator('body').getAttribute('data-wasm-error')}; browser errors: ${errors.join(', ')}`);
    }
    const screenshot = await canvas.screenshot();
    const pixel = await page.evaluate(async (encoded) => {
      const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
      const sample = document.createElement('canvas'); sample.width = image.width; sample.height = image.height;
      const context = sample.getContext('2d'); context.drawImage(image, 0, 0);
      return [...context.getImageData(Math.floor(image.width / 2), Math.floor(image.height / 2), 1, 1).data].slice(0, 3);
    }, screenshot.toString('base64'));
    assert.deepEqual(pixel, color);
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  }
  await picture(480, 540, [50, 200, 50]);
  for (let round = 0; round < 2; round++) {
    await canvas.press('Space', { delay: 80 }); await picture(320, 200, [220, 50, 50]);
    await canvas.press('f', { delay: 80 }); await picture(624, 600, [50, 100, 220]);
    await canvas.press('g', { delay: 80 }); await picture(480, 540, [50, 200, 50]);
  }
  assert.deepEqual(errors, []);
});
