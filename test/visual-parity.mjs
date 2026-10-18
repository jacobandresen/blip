import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage } from './lib/harness.mjs';

const GAMES = [
  ['ADDER', 'adder'], ['BOUNCER', 'bouncer'], ['BRAWLER', 'brawler'],
  ['BUBBLER', 'bubbler'], ['DEFENDER', 'galactic_defender'],
  ['METEORS', 'meteors'], ['RALLY', 'rally'], ['SERPENT', 'serpent'],
  ['SKY_RAIDER', 'sky_raider'],
];
const MOVE_KEY = { BRAWLER: 'd' };
const ARTIFACTS = process.env.BLIP_VISUAL_ARTIFACTS === '1';
const ARTIFACT_DIR = new URL('./artifacts/visual-parity/', import.meta.url);

async function frameStats(page, image) {
  return page.evaluate(async (encoded) => {
    const source = new Image();
    source.src = `data:image/png;base64,${encoded}`;
    await source.decode();
    const sample = document.createElement('canvas');
    sample.width = source.naturalWidth; sample.height = source.naturalHeight;
    const context = sample.getContext('2d');
    context.drawImage(source, 0, 0);
    const { data, width, height } = context.getImageData(0, 0, sample.width, sample.height);
    let bright = 0, samples = 0;
    for (let y = Math.floor(height * .1); y < height * .9; y += 3) {
      for (let x = Math.floor(width * .1); x < width * .9; x += 3) {
        const i = (y * width + x) * 4;
        if (data[i] + data[i + 1] + data[i + 2] > 240) bright++;
        samples++;
      }
    }
    return { lit: bright / samples, width, height };
  }, image.toString('base64'));
}

async function captureVisible(page, canvas, label) {
  let last;
  const started = Date.now();
  for (let attempt = 0; attempt < 80; attempt++) {
    const image = await canvas.screenshot();
    const stats = await frameStats(page, image);
    last = { image, stats };
    if (stats.lit > .002) return { ...last, waitMs: Date.now() - started };
    await page.waitForTimeout(100);
  }
  assert.fail(`${label} stayed blank (${last.stats.lit.toFixed(4)} lit pixels)`);
}

async function saveImage(slug, version, stage, image) {
  if (!ARTIFACTS) return;
  await mkdir(ARTIFACT_DIR, { recursive: true });
  await writeFile(new URL(`${slug}-${version}-${stage}.png`, ARTIFACT_DIR), image);
}

async function checkPlayFrames(page, canvas, label, slug, version) {
  let blanks = 0, first, last;
  for (let i = 0; i < 12; i++) {
    const image = await canvas.screenshot();
    const stats = await frameStats(page, image);
    if (stats.lit <= .002) blanks++;
    if (!first) first = stats;
    last = stats;
    if (i === 0 || i === 11) await saveImage(slug, version, `play-${i}`, image);
    await page.waitForTimeout(100);
  }
  assert.ok(blanks <= 1, `${label} showed ${blanks}/12 blank play frames`);
  return { first, last, blanks };
}

async function startDrawProbe(page, inFrame) {
  return page.evaluate((frame) => {
    const view = frame ? document.querySelector('#basic-wasm-frame').contentWindow : window;
    const gl = view.gl;
    if (!gl) return false;
    view.__blipDrawTimes = [];
    const wrap = (object, name) => {
      const original = object?.[name];
      if (!original || original.__blipProbe) return;
      const wrapped = function (...args) {
        if (view.__blipDrawTimes.length < 30000) view.__blipDrawTimes.push(view.performance.now());
        return original.apply(this, args);
      };
      wrapped.__blipProbe = true;
      object[name] = wrapped;
    };
    for (const name of ['drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced']) wrap(gl, name);
    const extension = gl.getExtension('ANGLE_instanced_arrays');
    for (const name of ['drawArraysInstancedANGLE', 'drawElementsInstancedANGLE']) wrap(extension, name);
    return true;
  }, inFrame);
}

function drawCadence(times) {
  const recent = times.filter((time) => time >= times.at(-1) - 1500);
  const batches = [];
  for (const time of recent) if (!batches.length || time - batches.at(-1) > 8) batches.push(time);
  const span = batches.at(-1) - batches[0];
  return span > 0 ? Math.round((batches.length - 1) * 1000 / span) : 0;
}

for (const [name, slug] of GAMES.filter(([name]) => !process.env.BLIP_VISUAL_GAME || process.env.BLIP_VISUAL_GAME === name)) {
  test(`${name}: BASIC and Rust render persistent play frames`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
    await page.emulateMedia({ reducedMotion: 'reduce' });

    await page.goto(`${origin}/basic.html`);
    await page.evaluate((game) => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit(`LOAD ${game}`); }, name);
    await page.waitForFunction((game) => document.querySelector('#basic-output').textContent.includes(`LOADED /TAPE/${game}.BLIP`), name);
    await page.evaluate(() => BlipBasic.submit('RUN'));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    const basicCanvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    const basicTitle = await captureVisible(page, basicCanvas, `${name} BASIC title`);
    assert.equal(await startDrawProbe(page, true), true);
    await saveImage(slug, 'basic', 'title', basicTitle.image);
    await basicCanvas.press('Space');
    await page.waitForTimeout(450);
    await captureVisible(page, basicCanvas, `${name} BASIC after Start`);
    await basicCanvas.press(MOVE_KEY[name] || 'ArrowRight');
    const basicPlay = await checkPlayFrames(page, basicCanvas, `${name} BASIC`, slug, 'basic');
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    const basicTimes = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.__blipDrawTimes || []);

    await page.goto(`${origin}/${slug}/index.html?amstrad=1`);
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), undefined, { timeout: 20000 });
    await page.waitForTimeout(900);
    const rustCanvas = page.locator('#glcanvas');
    const rustTitle = await captureVisible(page, rustCanvas, `${name} Rust title`);
    assert.equal(await startDrawProbe(page, false), true);
    await saveImage(slug, 'rust', 'title', rustTitle.image);
    await rustCanvas.press('Space');
    await page.waitForTimeout(450);
    if (name === 'BRAWLER') {
      await rustCanvas.press('Space');
      await page.waitForTimeout(450);
      await rustCanvas.press('Space');
      await page.waitForTimeout(450);
    }
    await captureVisible(page, rustCanvas, `${name} Rust after Start`);
    await rustCanvas.press(MOVE_KEY[name] || 'ArrowRight');
    const rustPlay = await checkPlayFrames(page, rustCanvas, `${name} Rust`, slug, 'rust');
    const rustTimes = await page.evaluate(() => window.__blipDrawTimes || []);

    const basicHz = drawCadence(basicTimes), rustHz = drawCadence(rustTimes);
    assert.ok(basicHz > 10 && rustHz > 10, `${name} draw cadence: BASIC ${basicHz} (${basicTimes.length} calls), Rust ${rustHz} (${rustTimes.length} calls)`);
    console.log(`${name}: title lit BASIC ${(basicTitle.stats.lit * 100).toFixed(1)}%, Rust ${(rustTitle.stats.lit * 100).toFixed(1)}%; play lit BASIC ${(basicPlay.last.lit * 100).toFixed(1)}%, Rust ${(rustPlay.last.lit * 100).toFixed(1)}%; draw batches/s BASIC ${basicHz}, Rust ${rustHz}; visible-title wait BASIC ${basicTitle.waitMs}ms, Rust ${rustTitle.waitMs}ms`);
  });
}
