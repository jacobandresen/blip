import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

test('Escape copies a running tape game and its graphics into USER, then opens its source', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/paint.js*', (route) => route.abort());
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => {
    BlipBasic.submit('CD /TAPE');
    BlipBasic.submit('LOAD BOUNCER');
  });
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('LOADED /TAPE/BOUNCER.BLIP'));
  await page.evaluate(() => BlipBasic.submit('RUN'));
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });

  const canvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
  await canvas.focus();
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.body.hasAttribute('data-editor'), undefined, { timeout: 15000 });

  const copied = await page.evaluate(() => {
    const fs = JSON.parse(localStorage.getItem('blip-basic-filesystem'));
    return {
      file: document.querySelector('#editor-file').textContent,
      source: fs.files['/USER/BOUNCER/BOUNCER.BLIP'],
      dirs: fs.dirs,
      assets: Object.keys(fs.files).filter((name) => name.startsWith('/USER/BOUNCER/ASSETS/')),
      asset: Object.values(fs.files).find((value) => typeof value === 'string' && value.startsWith('data:image/png;base64,')),
      currentFile: document.querySelector('#editor-file').getAttribute('data-path'),
    };
  });
  assert.ok(copied.assets.length > 0 && copied.assets.every((name) => name.endsWith('.PNG')), `the tape's pictures are copied, and only they; found ${copied.assets.length}, dirs=${copied.dirs.join(',')}`);
  assert.match(copied.file, /BOUNCER\.BLIP/);
  assert.ok(copied.source, 'the source is saved in USER');
  assert.match(copied.source, /\/USER\/BOUNCER\/ASSETS\//);
  assert.doesNotMatch(copied.source, /\/TAPE\/BOUNCER\//);
  assert.ok(copied.dirs.includes('/USER/BOUNCER'));
  assert.ok(copied.dirs.includes('/USER/BOUNCER/ASSETS'));
  assert.match(copied.asset, /^data:image\/png;base64,/);
  assert.equal(await page.locator('body').getAttribute('data-wasm-running'), null);
  assert.equal(await page.locator('body').getAttribute('data-editor'), '');

  await page.reload();
  await page.evaluate(() => BlipBasic.submit('CD /USER/BOUNCER'));
  await page.evaluate(() => BlipBasic.submit('DIR'));
  await page.waitForFunction(() => document.querySelector('#basic-output').textContent.includes('BOUNCER.BLIP'));
  const listing = await page.locator('#basic-output').textContent();
  assert.match(listing, /ASSETS/);
  assert.match(listing, /BOUNCER\.BLIP/);

  await page.evaluate(() => {
    const fs = JSON.parse(localStorage.getItem('blip-basic-filesystem'));
    fs.files['/USER/BOUNCER/BOUNCER.BLIP'] += '99999 REM USER EDIT\n';
    localStorage.setItem('blip-basic-filesystem', JSON.stringify(fs));
  });
  await page.reload();
  await page.evaluate(() => {
    BlipBasic.submit('CD /USER/BOUNCER');
    BlipBasic.submit('LOAD BOUNCER');
    BlipBasic.submit('RUN');
  });
  await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running'), undefined, { timeout: 15000 });
  await page.frameLocator('#basic-wasm-frame').locator('#glcanvas').focus();
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => document.body.hasAttribute('data-editor'), undefined, { timeout: 15000 });
  const preserved = await page.evaluate(() => JSON.parse(localStorage.getItem('blip-basic-filesystem')).files['/USER/BOUNCER/BOUNCER.BLIP']);
  assert.match(preserved, /99999 REM USER EDIT/);
});
