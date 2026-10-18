import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { openPage } from './lib/harness.mjs';

test('ROM INTERMISSION XM imports notes and plays non-silent audio', async (t) => {
  const { page, origin } = await openPage(t, 'chromium');
  const pageErrors = [];
  const dialogs = [];
  page.on('pageerror', (error) => pageErrors.push(error.message));
  page.on('dialog', async (dialog) => { dialogs.push(dialog.message()); await dialog.dismiss(); });
  await page.route('**/*.wasm', (route) => route.abort());
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('TUNE BLUE_INTERMISSION'));
  assert.match(await page.locator('#basic-output').textContent(), /TUNE TAKES NO FILE; USE IMPORT XM/);
  await page.evaluate(() => BlipBasic.submit('TUNE'));
  await page.locator('#tune').waitFor({ state: 'visible' });
  assert.equal(await page.locator('#tune-name').inputValue(), '/USER/MUSIC/SONG.TUNE');
  const xm = await readFile(fileURLToPath(new URL('../web/ROM/TUNES/BLUE_INTERMISSION.XM', import.meta.url)));
  await page.locator('#tune-file').evaluate((input, bytes) => {
    const file = new File([new Uint8Array(bytes)], 'BLUE_INTERMISSION.XM', { type: 'application/octet-stream' });
    const transfer = new DataTransfer();
    transfer.items.add(file);
    input.files = transfer.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
  }, Array.from(xm));
  await page.waitForFunction(() => document.querySelector('#tune-status').textContent.includes('IMPORTED'));

  const track = await page.evaluate(() => {
    const encoded = window.blipBasicTuneExport.waveData().split(',')[1];
    const binary = atob(encoded);
    const samples = new DataView(Uint8Array.from(binary, (char) => char.charCodeAt(0)).buffer);
    let peak = 0;
    for (let offset = 44; offset + 1 < samples.byteLength; offset += 2) {
      peak = Math.max(peak, Math.abs(samples.getInt16(offset, true)));
    }
    return {
      notes: document.querySelectorAll('#tune-grid .has-note').length,
      peak,
      samples: (samples.byteLength - 44) / 2,
    };
  });
  assert.ok(track.notes > 0, 'the XM should populate note cells');
  assert.ok(track.peak > 500, 'the imported tune should render audible WAV samples');
  assert.ok(track.samples > 0, 'the generated preview should contain audio samples');

  await page.evaluate(() => {
    const createSource = AudioContext.prototype.createBufferSource;
    AudioContext.prototype.createBufferSource = function () {
      const context = this;
      const source = createSource.call(context);
      const start = source.start.bind(source);
      source.start = (...args) => {
        window.__tunePlaybackState = context.state;
        return start(...args);
      };
      return source;
    };
    const connect = AudioNode.prototype.connect;
    AudioNode.prototype.connect = function (destination, ...args) {
      const result = connect.call(this, destination, ...args);
      if (destination === this.context.destination && !window.__tuneAnalyser) {
        const analyser = this.context.createAnalyser();
        analyser.fftSize = 2048;
        connect.call(this, analyser);
        window.__tuneAnalyser = analyser;
        window.__tuneOutputContext = this.context;
      }
      return result;
    };
  });
  await page.locator('#tune-play').click();
  await page.waitForFunction(() => document.querySelector('#tune-status').textContent.startsWith('PLAYING ·'));
  assert.equal(await page.evaluate(() => window.__tunePlaybackState), 'running', 'the audio source should start on a running context');
  await page.waitForFunction(() => {
    const analyser = window.__tuneAnalyser;
    if (!analyser || window.__tuneOutputContext?.state !== 'running') return false;
    const samples = new Float32Array(analyser.fftSize);
    analyser.getFloatTimeDomainData(samples);
    return Math.sqrt(samples.reduce((sum, sample) => sum + sample * sample, 0) / samples.length) > 0.01;
  }, undefined, { timeout: 5000 });
  assert.deepEqual(pageErrors, [], 'playback should not raise a browser error');
  assert.deepEqual(dialogs, [], 'playback should not show a browser alert');
  const visibleErrors = [
    await page.locator('#tune-status').textContent(),
    await page.locator('#basic-output').textContent(),
  ].join('\n');
  assert.doesNotMatch(visibleErrors, /UNEXPECTED TOKEN|PLAY ERROR/i, 'playback should not display a syntax or audio error');
});
