import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('PAINT stays in the monitor and saves game-size PNG textures in /USER', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1100, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('PAINT'));
  assert.equal(await page.locator('#paint-name').inputValue(), '/USER/GFX/SPRITE.PNG');
  await page.evaluate(() => BlipBasic.submit('PAINT "/TAPE/SERPENT/ASSETS/FOOD.PNG"'));
  assert.match(await page.locator('#paint-status').textContent(), /only saves in \/USER/i);
  assert.deepEqual(await page.locator('#paint').evaluate((el) => {
    const a=el.getBoundingClientRect(),b=document.querySelector('#basic-screen').getBoundingClientRect();
    return [a.left>=b.left,a.top>=b.top,a.right<=b.right+1,a.bottom<=b.bottom+1];
  }), [true,true,true,true]);
  assert.deepEqual(await page.locator('#paint-canvas').evaluate((el) => [el.width,el.height]), [320,200]);
  await page.locator('#paint-size').selectOption('64x64');
  assert.deepEqual(await page.locator('#paint-canvas').evaluate((el) => [el.width,el.height]), [64,64]);
  await page.locator('#paint-canvas').evaluate((el) => { const c=el.getContext('2d');c.fillStyle='#ff5696';c.fillRect(10,12,1,1); });
  await page.locator('#paint-save').click();
  const saved=await page.evaluate(() => ({raw:localStorage.getItem('blip-basic-filesystem'),status:document.querySelector('#paint-status').textContent}));
  assert.ok(saved.raw, JSON.stringify(saved));
  const image=await page.evaluate(async () => {
    const file=JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/GFX/SPRITE.PNG'];
    const img=new Image();img.src=file;await img.decode();
    const probe=document.createElement('canvas');probe.width=img.width;probe.height=img.height;
    const ctx=probe.getContext('2d');ctx.drawImage(img,0,0);return {prefix:file.slice(0,22),width:img.width,height:img.height,pixel:Array.from(ctx.getImageData(10,12,1,1).data)};
  });
  assert.equal(image.prefix, 'data:image/png;base64,');
  assert.deepEqual([image.width,image.height], [64,64]);
  assert.deepEqual(image.pixel, [255,86,150,255]);
  await page.locator('#paint-close').click();
  await page.evaluate(() => BlipBasic.submit('PAINT "/USER/GFX/SPRITE.PNG"'));
  await page.waitForFunction(() => document.querySelector('#paint-status').textContent.startsWith('OPEN'));
  assert.deepEqual(await page.locator('#paint-canvas').evaluate((el) => Array.from(el.getContext('2d').getImageData(10,12,1,1).data)), [255,86,150,255]);
  await page.locator('#paint-close').click();
  await page.evaluate(() => {
    BlipBasic.submit('10 DRAW_TEXTURE("/USER/GFX/SPRITE.PNG",0,0,320,200)');
    BlipBasic.submit('20 END');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.waitForTimeout(250);
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
});
