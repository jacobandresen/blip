import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { openPage, gameReady } from './lib/harness.mjs';
import { GAME_SLUGS } from './lib/games.mjs';

// Every game by its uppercase name and its slug.
const GAMES = GAME_SLUGS.map((slug) => [slug.toUpperCase(), slug]);
const MOVE_KEY = {
  ADDER: 'ArrowLeft', BOUNCER: 'ArrowLeft', BRAWLER: 'd', BUBBLER: 'ArrowRight',
  DEFENDER: 'ArrowLeft', METEORS: 'ArrowLeft', RALLY: 'ArrowUp',
  SERPENT: 'ArrowUp', RAIDER: 'ArrowUp', WARLOCK: 'ArrowDown',
};
const START_CHANGE_MIN = { METEORS: .02, RALLY: .01, SERPENT: .03 };
// At 85 px/s Adder adds about 120 trail pixels during the 350 ms input sample.
const RESPONSE_MIN = { ADDER: .00005 };
const ARTIFACTS = process.env.BLIP_VISUAL_ARTIFACTS === '1';
const ARTIFACT_DIR = new URL('./artifacts/visual/', import.meta.url);

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

async function snakeRows(page, image) {
  return page.evaluate(([encoded, bounds]) => {
    const source = new Image(); source.src = `data:image/png;base64,${encoded}`;
    return source.decode().then(() => {
      const canvas = document.createElement('canvas'); canvas.width = source.naturalWidth; canvas.height = source.naturalHeight;
      const context = canvas.getContext('2d'); context.drawImage(source, 0, 0);
      const { data, width, height } = context.getImageData(0, 0, canvas.width, canvas.height);
      // The rows the snake fills: five bright green pixels or more. The grid and border are dim,
      // and the apple's leaf never fills a row like that.
      let rows = 0;
      for (let y = Math.floor(height * bounds.y0); y < height * bounds.y1; y++) {
        let green = 0;
        for (let x = Math.floor(width * bounds.x0); x < width * bounds.x1; x++) {
          const i = (y * width + x) * 4, r = data[i], g = data[i + 1], b = data[i + 2];
          if (g > 110 && r < g * .72 && b < g * .82) green++;
        }
        if (green >= 5) rows++;
      }
      return rows;
    });
  }, [image.toString('base64'), { x0: .12, x1: .88, y0: .08, y1: .95 }]);
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

async function saveImage(slug, stage, image) {
  if (!ARTIFACTS) return;
  await mkdir(ARTIFACT_DIR, { recursive: true });
  await writeFile(new URL(`${slug}-${stage}.png`, ARTIFACT_DIR), image);
}

async function checkPlayFrames(page, canvas, label, slug) {
  let blanks = 0, first, last;
  for (let i = 0; i < 12; i++) {
    const image = await canvas.screenshot();
    const stats = await frameStats(page, image);
    if (stats.visible <= .0001) blanks++;
    if (!first) first = stats;
    last = stats;
    if (i === 0 || i === 11) await saveImage(slug, `play-${i}`, image);
    await page.waitForTimeout(100);
  }
  assert.ok(blanks <= 1, `${label} showed ${blanks}/12 blank play frames`);
  return { first, last, blanks };
}

async function startDrawProbe(page) {
  return page.evaluate(() => {
    const view = window;
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
  });
}

function drawCadence(times) {
  const recent = times.filter((time) => time >= times.at(-1) - 1500);
  const batches = [];
  for (const time of recent) if (!batches.length || time - batches.at(-1) > 8) batches.push(time);
  const span = batches.at(-1) - batches[0];
  return span > 0 ? Math.round((batches.length - 1) * 1000 / span) : 0;
}

for (const [name, slug] of GAMES.filter(([name]) => !process.env.BLIP_VISUAL_GAME || process.env.BLIP_VISUAL_GAME === name)) {
  test(`${name} enters play and responds to movement`, async (t) => {
    const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 1000, height: 900 } });
    await page.emulateMedia({ reducedMotion: 'reduce' });

    // Free play with no deck: the picture alone.
    await page.goto(`${origin}/${slug}/index.html?app`);
    await gameReady(page);
    await page.waitForTimeout(900);
    const canvas = page.locator('#glcanvas');
    const title = await captureVisible(page, canvas, `${name} title`);
    assert.equal(await startDrawProbe(page), true);
    await saveImage(slug, 'title', title.image);
    await canvas.click({ position: { x: 12, y: 12 } });
    await canvas.evaluate((element) => element.focus());
    await page.keyboard.down('Space');
    await page.waitForTimeout(150);
    await page.keyboard.up('Space');
    await page.waitForTimeout(name === 'ADDER' ? 20 : name === 'SERPENT' ? 120 : 450);
    if (name === 'RALLY') {
      await tapKey(canvas, 'Enter');
      await page.waitForTimeout(650);
    }
    if (name === 'BOUNCER') {
      await tapKey(canvas, 'Space');
      await tapKey(canvas, 'ArrowUp');
      await page.waitForTimeout(250);
    }
    if (name === 'BRAWLER') {
      // Demo, fighter select, versus screen.
      for (let i = 0; i < 3; i++) {
        await tapKey(canvas, 'Space');
        await page.waitForTimeout(450);
      }
    }
    const started = await captureVisible(page, canvas, `${name} after Start`);
    const startChange = await frameDifference(page, title.image, started.image);
    await saveImage(slug, 'started', started.image);
    assert.ok(startChange > (START_CHANGE_MIN[name] || .05), `${name} stayed on its title after Start (${(startChange * 100).toFixed(2)}% changed)`);
    const before = await canvas.screenshot();
    if (name === 'SERPENT') {
      await tapKey(canvas, MOVE_KEY[name]);
      await page.waitForTimeout(240);
    } else {
      await holdKey(page, canvas, MOVE_KEY[name], 350);
    }
    const after = await canvas.screenshot();
    const response = await frameDifference(page, before, after);
    if (name === 'SERPENT') {
      const rowsBefore = await snakeRows(page, before), rowsAfter = await snakeRows(page, after);
      assert.ok(rowsAfter > rowsBefore + 12, `SERPENT did not turn upward: the snake fills ${rowsBefore} rows, then ${rowsAfter}`);
    }
    assert.ok(response > (RESPONSE_MIN[name] || .0005), `${name} did not visibly respond to ${MOVE_KEY[name]} (${response.toFixed(4)} changed pixels)`);
    await saveImage(slug, 'input-before', before);
    await saveImage(slug, 'input-after', after);
    const play = await checkPlayFrames(page, canvas, name, slug);
    const hz = drawCadence(await page.evaluate(() => window.__blipDrawTimes || []));
    assert.ok(hz > 10, `${name} draw cadence ${hz}`);
    console.log(`${name}: ${MOVE_KEY[name]} changed ${(response * 100).toFixed(2)}%; title lit ${(title.stats.lit * 100).toFixed(1)}%; play lit ${(play.last.lit * 100).toFixed(1)}%; draw batches/s ${hz}; visible-title wait ${title.waitMs}ms`);
  });
}
