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
const MOVE_KEY = {
  ADDER: 'ArrowLeft', BOUNCER: 'ArrowLeft', BRAWLER: 'd', BUBBLER: 'ArrowRight',
  DEFENDER: 'ArrowLeft', METEORS: 'ArrowLeft', RALLY: 'ArrowUp',
  SERPENT: 'ArrowUp', SKY_RAIDER: 'ArrowUp',
};
const START_CHANGE_MIN = { METEORS: .02, RALLY: .01, SERPENT: .03 };
// At 85 px/s Adder adds about 120 trail pixels during the 350 ms input sample.
const RESPONSE_MIN = { ADDER: .00005 };
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
    let bright = 0, visible = 0, samples = 0;
    for (let y = Math.floor(height * .1); y < height * .9; y += 3) {
      for (let x = Math.floor(width * .1); x < width * .9; x += 3) {
        const i = (y * width + x) * 4;
        const light = data[i] + data[i + 1] + data[i + 2];
        if (light > 240) bright++;
        if (light > 45) visible++;
        samples++;
      }
    }
    return { lit: bright / samples, visible: visible / samples, width, height };
  }, image.toString('base64'));
}

async function frameDifference(page, before, after) {
  return page.evaluate(([left, right]) => {
    const decode = async (encoded) => {
      const image = new Image(); image.src = `data:image/png;base64,${encoded}`; await image.decode();
      const canvas = document.createElement('canvas'); canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
      const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
      return context.getImageData(0, 0, canvas.width, canvas.height).data;
    };
    return Promise.all([decode(left), decode(right)]).then(([a, b]) => {
      let changed = 0, total = 0;
      for (let i = 0; i < a.length; i += 16) {
        const delta = Math.abs(a[i] - b[i]) + Math.abs(a[i + 1] - b[i + 1]) + Math.abs(a[i + 2] - b[i + 2]);
        if (delta > 60) changed++;
        total++;
      }
      return changed / total;
    });
  }, [before.toString('base64'), after.toString('base64')]);
}

async function greenBounds(page, image, version) {
  return page.evaluate(([encoded, bounds]) => {
    const source = new Image(); source.src = `data:image/png;base64,${encoded}`;
    return source.decode().then(() => {
      const canvas = document.createElement('canvas'); canvas.width = source.naturalWidth; canvas.height = source.naturalHeight;
      const context = canvas.getContext('2d'); context.drawImage(source, 0, 0);
      const { data, width, height } = context.getImageData(0, 0, canvas.width, canvas.height);
      let left = width, right = 0, top = height, bottom = 0;
      for (let y = Math.floor(height * bounds.y0); y < height * bounds.y1; y++) {
        for (let x = Math.floor(width * bounds.x0); x < width * bounds.x1; x++) {
          const i = (y * width + x) * 4, r = data[i], g = data[i + 1], b = data[i + 2];
          if (g > 60 && r < g * .72 && b < g * .82) { left = Math.min(left, x); right = Math.max(right, x); top = Math.min(top, y); bottom = Math.max(bottom, y); }
        }
      }
      return right ? { left, right, top, bottom, width: right - left + 1, height: bottom - top + 1 } : null;
    });
  }, [image.toString('base64'), version === 'basic' ? { x0: .26, x1: .74, y0: .21, y1: .85 } : { x0: .12, x1: .88, y0: .08, y1: .95 }]);
}

async function holdKey(page, canvas, key, ms, focusSelector) {
  if (focusSelector) await page.locator(focusSelector).focus();
  else await canvas.evaluate((element) => element.focus());
  await page.keyboard.down(key);
  await page.waitForTimeout(ms);
  await page.keyboard.up(key);
}

async function tapKey(canvas, key) {
  await canvas.press(key, { delay: 100 });
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
    if (stats.visible <= .0001) blanks++;
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
  test(`${name}: BASIC and Rust enter play and respond to movement`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
    const browserErrors = [];
    page.on('pageerror', (error) => browserErrors.push(error.message));
    await page.emulateMedia({ reducedMotion: 'reduce' });

    await page.route('**/paint.js*', (route) => route.abort());
    await page.goto(`${origin}/basic.html`);
    await page.evaluate((game) => { BlipBasic.submit('CD /TAPE'); BlipBasic.submit(`LOAD ${game}`); }, name);
    await page.waitForFunction((game) => document.querySelector('#basic-output').textContent.includes(`LOADED /TAPE/${game}.BLIP`), name);
    await page.evaluate(() => BlipBasic.submit('RUN'));
    try {
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'), undefined, { timeout: 20000 });
    } catch {
      const debug = await page.evaluate(() => ({ output: document.querySelector('#basic-output').textContent, frame: document.querySelector('#basic-wasm-frame').getAttribute('src'), ready: document.body.getAttribute('data-wasm-playing'), error: document.body.getAttribute('data-wasm-error'), frameBody: document.querySelector('#basic-wasm-frame').contentDocument?.body?.innerText }));
      assert.fail(`${name} BASIC did not start: ${JSON.stringify({ ...debug, browserErrors })}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    const basicCanvas = page.frameLocator('#basic-wasm-frame').locator('#glcanvas');
    const basicTitle = await captureVisible(page, basicCanvas, `${name} BASIC title`);
    assert.equal(await startDrawProbe(page, true), true);
    await page.evaluate(() => {
      const frame = document.querySelector('#basic-wasm-frame'), canvas = frame.contentDocument.querySelector('#glcanvas');
      frame.contentWindow.__parityKeys = [];
      canvas.addEventListener('keydown', (event) => frame.contentWindow.__parityKeys.push([event.key, event.code]));
    });
    await saveImage(slug, 'basic', 'title', basicTitle.image);
    await basicCanvas.evaluate((canvas) => canvas.focus());
    await tapKey(basicCanvas, 'Space');
    await page.waitForTimeout(name === 'ADDER' ? 20 : name === 'SERPENT' ? 120 : 450);
    if (name === 'BRAWLER') await page.waitForTimeout(1650);
    if (name === 'RALLY') {
      await tapKey(basicCanvas, 'Space');
      await page.waitForTimeout(650);
    }
    if (name === 'BOUNCER') {
      await tapKey(basicCanvas, 'Space');
      await tapKey(basicCanvas, 'ArrowUp');
      await page.waitForTimeout(250);
    }
    const basicStarted = await captureVisible(page, basicCanvas, `${name} BASIC after Start`);
    const basicStartChange = await frameDifference(page, basicTitle.image, basicStarted.image);
    const startChangeMin = START_CHANGE_MIN[name] || .05;
    assert.ok(basicStartChange > startChangeMin, `${name} BASIC stayed on its title after Start (${(basicStartChange * 100).toFixed(2)}% changed)`);
    const basicBefore = await basicCanvas.screenshot();
    if (name === 'SERPENT') {
      await tapKey(basicCanvas, MOVE_KEY[name]);
      await page.waitForTimeout(240);
    } else {
      await holdKey(page, basicCanvas, MOVE_KEY[name], 350);
    }
    const basicAfter = await basicCanvas.screenshot();
    const basicResponse = await frameDifference(page, basicBefore, basicAfter);
    const basicInputTrace = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.__parityKeys);
    if (name === 'SERPENT') {
      const before = await greenBounds(page, basicBefore, 'basic'), after = await greenBounds(page, basicAfter, 'basic');
      assert.ok(before && after && after.height > before.height + 12, `SERPENT BASIC did not turn upward: ${JSON.stringify({ before, after })}`);
    }
    assert.ok(basicResponse > (RESPONSE_MIN[name] || .0005), `${name} BASIC did not visibly respond to ${MOVE_KEY[name]} (${basicResponse.toFixed(4)} changed pixels)`);
    await saveImage(slug, 'basic', 'input-before', basicBefore);
    await saveImage(slug, 'basic', 'input-after', basicAfter);
    const basicPlay = await checkPlayFrames(page, basicCanvas, `${name} BASIC`, slug, 'basic');
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    const basicTimes = await page.evaluate(() => document.querySelector('#basic-wasm-frame').contentWindow.__blipDrawTimes || []);

    await page.goto(`${origin}/${slug}/index.html?amstrad=1`);
    await page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), undefined, { timeout: 20000 });
    if (await page.locator('#need-coin-overlay').evaluate((overlay) => overlay.classList.contains('visible'))) {
      await page.evaluate(() => document.getElementById('insert-coin-btn').click());
      await page.waitForFunction(() => !document.querySelector('#need-coin-overlay').classList.contains('visible'));
    }
    await page.waitForTimeout(900);
    const rustCanvas = page.locator('#glcanvas');
    const rustTitle = await captureVisible(page, rustCanvas, `${name} Rust title`);
    assert.equal(await startDrawProbe(page, false), true);
    await saveImage(slug, 'rust', 'title', rustTitle.image);
    await rustCanvas.click({ position: { x: 12, y: 12 } });
    await rustCanvas.evaluate((canvas) => canvas.focus());
    await page.keyboard.down('Space');
    await page.waitForTimeout(150);
    await page.keyboard.up('Space');
    await page.waitForTimeout(name === 'ADDER' ? 20 : name === 'SERPENT' ? 120 : 450);
    if (name === 'RALLY') {
      await tapKey(rustCanvas, 'Enter');
      await page.waitForTimeout(650);
    }
    if (name === 'BOUNCER') {
      await tapKey(rustCanvas, 'Space');
      await tapKey(rustCanvas, 'ArrowUp');
      await page.waitForTimeout(250);
    }
    if (name === 'BRAWLER') {
      await tapKey(rustCanvas, 'Space');
      await page.waitForTimeout(450);
      await tapKey(rustCanvas, 'Space');
      await page.waitForTimeout(450);
      await tapKey(rustCanvas, 'Space');
      await page.waitForTimeout(450);
    }
    const rustStarted = await captureVisible(page, rustCanvas, `${name} Rust after Start`);
    const rustStartChange = await frameDifference(page, rustTitle.image, rustStarted.image);
    await saveImage(slug, 'rust', 'started', rustStarted.image);
    assert.ok(rustStartChange > startChangeMin, `${name} Rust stayed on its title after Start (${(rustStartChange * 100).toFixed(2)}% changed)`);
    const rustBefore = await rustCanvas.screenshot();
    await holdKey(page, rustCanvas, MOVE_KEY[name], 350);
    const rustAfter = await rustCanvas.screenshot();
    const rustResponse = await frameDifference(page, rustBefore, rustAfter);
    assert.ok(rustResponse > (RESPONSE_MIN[name] || .0005), `${name} Rust did not visibly respond to ${MOVE_KEY[name]} (${rustResponse.toFixed(4)} changed pixels)`);
    await saveImage(slug, 'rust', 'input-before', rustBefore);
    await saveImage(slug, 'rust', 'input-after', rustAfter);
    const rustPlay = await checkPlayFrames(page, rustCanvas, `${name} Rust`, slug, 'rust');
    const rustTimes = await page.evaluate(() => window.__blipDrawTimes || []);

    const basicHz = drawCadence(basicTimes), rustHz = drawCadence(rustTimes);
    assert.ok(basicHz > 10 && rustHz > 10, `${name} draw cadence: BASIC ${basicHz} (${basicTimes.length} calls), Rust ${rustHz} (${rustTimes.length} calls)`);
    console.log(`${name}: ${MOVE_KEY[name]} changed BASIC ${(basicResponse * 100).toFixed(2)}%, Rust ${(rustResponse * 100).toFixed(2)}%; BASIC keys ${JSON.stringify(basicInputTrace)}; title lit BASIC ${(basicTitle.stats.lit * 100).toFixed(1)}%, Rust ${(rustTitle.stats.lit * 100).toFixed(1)}%; play lit BASIC ${(basicPlay.last.lit * 100).toFixed(1)}%, Rust ${(rustPlay.last.lit * 100).toFixed(1)}%; draw batches/s BASIC ${basicHz}, Rust ${rustHz}; visible-title wait BASIC ${basicTitle.waitMs}ms, Rust ${rustTitle.waitMs}ms`);
  });
}
