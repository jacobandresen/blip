import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

for (const touch of [false, true]) test(`TAPE mounts only the inserted cassette and keeps USER files after eject (${touch ? 'touch' : 'PC'})`, async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: touch, viewport: touch ? { width: 390, height: 844 } : { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  async function command(text) {
    await page.evaluate(async (text) => { await BlipBasic.submit(text); }, text);
  }
  const available = (path) => page.evaluate((path) => BlipBasic.isFileAvailable(path), path);
  await command('DIR /TAPE');
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('DIRECTORY OF /TAPE\n<EMPTY>'));
  assert.equal(await available('/TAPE/BOUNCER.BLIP'), false);
  assert.equal(await page.evaluate(() => BlipBasic.isDirectory('/TAPE/GFX')), false);

  await command('LOAD /TAPE/BOUNCER');
  await command('DIR /TAPE');
  const listing = await page.locator('#basic-output').textContent();
  assert.equal(listing.split('DIRECTORY OF /TAPE\n').at(-1).split('READY')[0].trim(), 'BOUNCER.BLIP\nGFX/');
  await command('DIR /TAPE/GFX');
  assert.equal(await available('/TAPE/BOUNCER.BLIP'), true);
  assert.equal(await available('/TAPE/GFX/BOUNCER/BALL.PNG'), true);
  assert.equal(await available('/TAPE/GFX/RALLY/BALL.PNG'), false);
  assert.equal(await page.evaluate(() => BlipBasic.isDirectory('/TAPE/GFX/RALLY')), false);
  await command('SAVE /USER/MY_BOUNCER');

  await command('CD /TAPE/GFX/BOUNCER');
  await command('LOAD /TAPE/RALLY');
  await command('DIR');
  const swapped = await page.locator('#basic-output').textContent();
  assert.equal(swapped.split('DIRECTORY OF /TAPE\n').at(-1).split('READY')[0].trim(), 'GFX/\nRALLY.BLIP');
  assert.equal(await available('/TAPE/BOUNCER.BLIP'), false);
  assert.equal(await available('/TAPE/RALLY.BLIP'), true);
  assert.equal(await available('/TAPE/GFX/BOUNCER/BALL.PNG'), false);
  await command('SOURCE /TAPE/BOUNCER');
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('TAPE NOT INSERTED'));
  await command('CD /TAPE/GFX/RALLY');
  const eject = page.getByRole('button', { name: 'Stop and eject tape' });
  if (touch) await eject.tap(); else await eject.click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-deck-tape'));
  assert.equal(await available('/TAPE/RALLY.BLIP'), false);
  await command('DIR /TAPE');
  await command('LOAD /USER/MY_BOUNCER');
  assert.ok(await page.evaluate(() => BlipBasic.lines.size > 20));
  assert.equal(await available('/TAPE/BOUNCER.BLIP'), false, 'loading a saved source does not insert its original cassette');
  assert.equal(await page.evaluate(() => BlipBasic.isDirectory('/TAPE/GFX')), false);
});

test('cached graphics from another cassette cannot be drawn', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async () => {
    await BlipBasic.submit('LOAD /TAPE/BOUNCER');
    await BlipBasic.submit('LOAD /TAPE/RALLY');
    BlipBasic.lines.clear();
    BlipBasic.submit('10 DRAW_TEXTURE("/TAPE/GFX/BOUNCER/BALL.PNG",0,0,8,8)');
    BlipBasic.submit('20 WAIT 0:GOTO 10');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.getAttribute('data-wasm-error')?.includes('File not available'));
  assert.match(await page.locator('body').getAttribute('data-wasm-error'), /BOUNCER\/BALL.PNG/);
});

test('a copied game keeps its exported pictures after the cassette is ejected', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(async () => { await BlipBasic.submit('LOAD /TAPE/BOUNCER'); BlipBasic.submit('RUN'); });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').press('Escape');
  await page.locator('#basic-editor').waitFor({ state: 'visible' });
  await page.getByRole('button', { name: 'Stop and eject tape' }).click();
  await page.waitForFunction(() => !document.body.hasAttribute('data-deck-tape'));
  await page.evaluate(async () => { await BlipBasic.submit('LOAD /USER/BOUNCER/BOUNCER'); BlipBasic.submit('RUN'); });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'));
  assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  assert.equal(await page.evaluate(() => BlipBasic.isFileAvailable('/TAPE/GFX/BOUNCER/BALL.PNG')), false);
  assert.ok(await page.evaluate(() => BlipBasic.filesystem.files['/USER/BOUNCER/GFX/BALL.PNG']?.startsWith('data:image/png')));
});
