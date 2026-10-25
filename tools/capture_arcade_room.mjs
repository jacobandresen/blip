#!/usr/bin/env node
// Screenshots of the arcade room (web/index.html) for review beside the photographs in issue #65.
//
//   node tools/capture_arcade_room.mjs [--still] [--lite] [--guides] [--perf] [--out <dir>]
//   node tools/capture_arcade_room.mjs --still --perf --runs 3 --trace [--baseline <commit>]
//   node tools/capture_arcade_room.mjs --still --soak [--soak-minutes 30] [--baseline <commit>]
//
// Four configurations (phone portrait, touch landscape, desktop, kiosk) × two aisle positions (at rest on
// Rally, and held exactly halfway to Bouncer) × two credit states (none, one) = 16 PNGs per mode, in
// test/artifacts/arcade-room/<full|lite>/. Each run also rewrites metrics.json for its mode and the gallery
// test/artifacts/arcade-room/index.html, which shows every mode captured so far.
//
//   --still   freezes every animation at a fixed time once the nearby attract loops have loaded, so two runs
//             of the same page give the same pictures
//   --lite    opens the page with ?room=lite
//   --guides  draws the room's shared regions (arcade-room.css --room-* tokens) over each picture
//   --perf    instead of pictures, replays a fixed touch drag on the phone and writes perf.json: layouts,
//             style recalculations and their time per touchmove, and frame intervals
//   --trace   saves a compressed CDP trace for each performance run
//   --warm-screens visits every machine and returns before measuring, separating first image loads from walking
//   --baseline serves room sources from a commit over the same server/assets, with no checkout
//   --soak    samples kiosk heap once a minute, identical double GC at minute 1/end, then a 30s frame trace
//
// Needs Node 18+, the repo's Playwright package and Chromium: $BLIP_CHROMIUM, else `chromium` on PATH, else
// Playwright's own (`npx playwright install chromium`). It serves web/ itself on port 8197.
// Historical --baseline runs also need Git on PATH.

import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { gzipSync } from 'node:zlib';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createFileServer, listenOn } from '../test/lib/harness.mjs';
import { chromiumBinary } from '../test/lib/chromium-binary.mjs';
import { holdHalfway, ROOM_SOURCES as DEFAULT_ROOM_SOURCES } from '../test/lib/arcade-room.mjs';

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const { chromium } = createRequire(import.meta.url)('playwright');
const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const outIndex = args.indexOf('--out');
const OUT = outIndex >= 0 ? path.resolve(args[outIndex + 1]) : path.join(ROOT, 'test/artifacts/arcade-room');
const MODE = flag('--lite') ? 'lite' : 'full';
const option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const BASELINE = option('--baseline', null);
if (BASELINE && !/^[a-f\d]{7,40}$/i.test(BASELINE)) throw new Error('--baseline must be a hexadecimal commit ID');
const RUNS = Number(option('--runs', 1)), SOAK_MINUTES = Number(option('--soak-minutes', 30));
if (!Number.isInteger(RUNS) || RUNS < 1 || RUNS > 10) throw new Error('--runs must be 1–10');
if (!Number.isInteger(SOAK_MINUTES) || SOAK_MINUTES < 1) throw new Error('--soak-minutes must be a positive integer');
const PORT = 8197;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const changedSources = BASELINE ? execFileSync('git', ['diff', '--name-only', BASELINE, '--', 'web'], { cwd: ROOT })
  .toString().trim().split('\n').filter((file) => /\.(html|css|js|json|svg)$/.test(file)).map((file) => file.slice(4)) : [];
const ROOM_SOURCES = [...new Set([...DEFAULT_ROOM_SOURCES, ...changedSources])];
const baselineSources = Object.fromEntries(BASELINE ? ROOM_SOURCES.flatMap((file) => {
  try { execFileSync('git', ['cat-file', '-e', `${BASELINE}:web/${file}`], { cwd: ROOT, stdio: 'ignore' }); }
  catch { return []; }
  return [[`/${file}`, execFileSync('git', ['show', `${BASELINE}:web/${file}`], { cwd: ROOT }).toString()]];
}) : []);

function sourceHash() {
  const hash = createHash('sha256');
  for (const file of ROOM_SOURCES) hash.update(file).update(BASELINE ? baselineSources[`/${file}`] ?? '(absent)' : fs.readFileSync(path.join(ROOT, 'web', file)));
  return hash.digest('hex');
}
const SOURCE_HASH = sourceHash();

const CONFIGS = [
  { name: 'phone', width: 390, height: 844, touch: true, layout: 'upright' },
  { name: 'landscape', width: 844, height: 390, touch: true, layout: 'landscape' },
  { name: 'desktop', width: 1280, height: 800, touch: false, layout: 'upright' },
  { name: 'kiosk', width: 1080, height: 1920, touch: false, layout: 'upright', fullscreen: true },
];
const POSITIONS = ['rest', 'half'];
const CREDITS = [0, 1];
// The photographs the room is drawn from (issue #65), linked rather than copied.
const REFERENCES = [
  ['R1 entrance', 'https://cdn.masto.host/mastodongamedevplace/media_attachments/files/113/753/045/647/958/720/original/67d31b64e0ba39a1.jpg'],
  ['R2 tiles', 'https://cdn.masto.host/mastodongamedevplace/media_attachments/files/113/753/045/634/490/380/original/c12205a4a0dfe15f.jpg'],
  ['R3 mural and carpet', 'https://cdn.masto.host/mastodongamedevplace/media_attachments/files/113/753/045/649/075/622/original/52f47bfd32ca5c17.jpg'],
  ['R4 entrance close-up', 'https://upload.wikimedia.org/wikipedia/commons/thumb/4/49/Video_Game_Museum_in_Berlin_%2845946108581%29_%28Arcade_Focus%29.jpg/960px-Video_Game_Museum_in_Berlin_%2845946108581%29_%28Arcade_Focus%29.jpg'],
];
const CHECKLIST = [
  'A warm, enclosed room: orange/red wall midtones, not a dark void',
  'Ceiling trim and sign read as lit materials (red fascia, yellow underside, neon tube)',
  'Large glossy mosaic blocks with pale grout',
  'A recognizable illustrated cabinet mural',
  'Offset red/black carpet meeting the wall at a warm step',
  'Edge decals where the layout allows them',
  'The focused cabinet, its screen, title, coin door and Start stay readable',
  'Lite keeps the palette and materials; check both at actual display size',
];

function gitCommit() {
  try {
    if (BASELINE) return execFileSync('git', ['rev-parse', BASELINE], { cwd: ROOT }).toString().trim();
    const head = execFileSync('git', ['rev-parse', '--short', 'HEAD'], { cwd: ROOT }).toString().trim();
    const dirty = execFileSync('git', ['status', '--porcelain', '--', 'web'], { cwd: ROOT }).toString().trim();
    return dirty ? `${head}+changes` : head;
  } catch { return 'unknown'; }
}

async function openArcade(browser, config, query = '') {
  const context = await browser.newContext({
    viewport: { width: config.width, height: config.height }, hasTouch: config.touch, deviceScaleFactor: 1,
  });
  if (config.fullscreen) await context.addInitScript(() => localStorage.setItem('blip-fullscreen', '1'));
  const page = await context.newPage();
  page.on('pageerror', (e) => console.error(`  page error: ${e.message}`));
  await page.goto(`${ORIGIN}/index.html${query}`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
  await page.evaluate(() => document.fonts.ready);
  const state = await page.evaluate(() => ({
    layout: document.documentElement.dataset.layout,
    fullscreen: document.documentElement.hasAttribute('data-fullscreen'),
  }));
  if (state.layout !== config.layout) throw new Error(`${config.name}: layout is ${state.layout}, not ${config.layout}`);
  if (!!config.fullscreen !== state.fullscreen) throw new Error(`${config.name}: kiosk fullscreen is ${state.fullscreen}`);
  return { context, page };
}

// Every animation, here and in the decks' frames: finite ones finished, endless ones (attract loops, blinks)
// paused one second in.
async function freeze(page) {
  await page.waitForFunction(() => [...document.querySelectorAll('#game-grid .card[data-loop]')]
    .every((c) => c.hasAttribute('data-loop-ready')), null, { timeout: 60000 });
  for (const frame of page.frames()) {
    await frame.evaluate(() => {
      for (const a of document.getAnimations()) {
        const timing = a.effect && a.effect.getComputedTiming();
        if (timing && timing.endTime === Infinity) { a.pause(); a.currentTime = 1000; } else { try { a.finish(); } catch (e) { a.pause(); } }
      }
    }).catch(() => {});
  }
}

async function measure(page) {
  return page.evaluate(() => {
    const grid = document.getElementById('game-grid');
    const g = grid.getBoundingClientRect();
    const clip = (r) => {
      const left = Math.max(r.left, g.left), top = Math.max(r.top, g.top);
      const right = Math.min(r.right, g.right), bottom = Math.min(r.bottom, g.bottom);
      return right > left && bottom > top ? { x: left, y: top, width: right - left, height: bottom - top } : null;
    };
    const layers = [...document.querySelectorAll('[data-room-layer]')].map((el) => {
      const cs = getComputedStyle(el);
      return {
        layer: el.dataset.roomLayer, zone: el.dataset.zone || '',
        shown: cs.display !== 'none' && cs.visibility !== 'hidden' && Number(cs.opacity) > 0,
        visible: clip(el.getBoundingClientRect()),
      };
    });
    const focused = document.querySelector('.card-focused');
    const css = getComputedStyle(grid);
    const regions = {};
    for (const name of ['trim', 'sign', 'wall', 'floor']) {
      regions[name] = [css.getPropertyValue(`--room-${name}-top`).trim(), css.getPropertyValue(`--room-${name}-bottom`).trim()];
    }
    regions.band = [css.getPropertyValue('--room-wall-top').trim(), css.getPropertyValue('--room-band-bottom').trim()];
    // How much of each region the cabinets leave in view: a 48×40 grid of points, each either on a cabinet or
    // through to the room (the room's layers take no pointer events).
    const seen = {};
    for (const name of Object.keys(regions)) {
      const top = parseFloat(regions[name][0]) / 100, bottom = parseFloat(regions[name][1]) / 100;
      let open = 0, all = 0;
      for (let r = 0; r < 40; r++) {
        const fy = (r + 0.5) / 40;
        if (fy < top || fy > bottom) continue;
        for (let c = 0; c < 48; c++) {
          const hit = document.elementFromPoint(g.left + (c + 0.5) * g.width / 48, g.top + fy * g.height);
          all++;
          if (hit && !hit.closest('.card') && hit.closest('.screen-bezel')) open++;
        }
      }
      seen[name] = all ? Math.round(100 * open / all) : null;
    }
    return {
      seen,
      layout: document.documentElement.dataset.layout,
      fullscreen: document.documentElement.hasAttribute('data-fullscreen'),
      close: grid.hasAttribute('data-close'),
      room: document.documentElement.dataset.room || '(none)',
      credit: document.documentElement.hasAttribute('data-credit'),
      cabinet: focused && focused.dataset.gameUrl.split('/')[0],
      aislePos: Number(grid.dataset.aislePos),
      roomTravel: grid.dataset.roomTravel === undefined ? null : Number(grid.dataset.roomTravel),
      grid: { x: g.left, y: g.top, width: g.width, height: g.height },
      regions, layers,
    };
  });
}

// The shared regions as labelled bands, for the composition review. Not part of the page.
async function drawGuides(page) {
  await page.evaluate(() => {
    const grid = document.getElementById('game-grid');
    const css = getComputedStyle(grid);
    const colours = { trim: '#ffe24a', sign: '#5fd0ff', wall: '#7dff7a', floor: '#ff7ad8', band: '#ffffff' };
    for (const [name, colour] of Object.entries(colours)) {
      const top = css.getPropertyValue(`--room-${name === 'band' ? 'wall' : name}-top`).trim();
      const bottom = css.getPropertyValue(`--room-${name}-bottom`).trim();
      if (!top || !bottom) continue;
      const band = document.createElement('div');
      band.setAttribute('aria-hidden', 'true');
      band.style.cssText = `position:absolute;left:0;right:0;top:${top};height:calc(${bottom} - ${top});z-index:999;pointer-events:none;` +
        `outline:2px dashed ${colour};outline-offset:-2px;background:${colour}14;font:bold 12px monospace;color:${colour};` +
        'text-shadow:0 1px 2px #000;padding:3px 6px;box-sizing:border-box';
      band.textContent = `${name} ${top}–${bottom}`;
      grid.appendChild(band);
    }
  });
}

async function capture(browser) {
  const dir = path.join(OUT, MODE);
  fs.mkdirSync(dir, { recursive: true });
  const shots = [];
  for (const config of CONFIGS) {
    for (const credits of CREDITS) {
      for (const position of POSITIONS) {
        const { context, page } = await openArcade(browser, config, `?room=${MODE}`);
        for (let i = 0; i < credits; i++) await page.keyboard.press('5');
        if (credits) await page.waitForFunction(() => document.documentElement.hasAttribute('data-credit'));
        if (position === 'half') await holdHalfway(page);
        // Lamps, the coin door and any room transitions settle.
        await page.waitForTimeout(1600);
        if (flag('--still')) await freeze(page);
        if (flag('--guides')) await drawGuides(page);
        const file = `${config.name}-${position}-${credits ? 'credit' : 'nocredit'}.png`;
        await page.screenshot({ path: path.join(dir, file), timeout: 60000 });
        const m = await measure(page);
        if (m.room !== MODE) throw new Error(`${file}: expected ${MODE}, got ${m.room}`);
        if (credits && !m.credit) throw new Error(`${file}: the credit did not register`);
        shots.push({ file: `${MODE}/${file}`, config: config.name, viewport: `${config.width}×${config.height}`, dpr: 1,
          position, credits, mode: MODE, ...m });
        console.log(`  ${MODE}/${file}  ${m.cabinet} @ ${m.aislePos}  layout=${m.layout}${m.fullscreen ? ' kiosk' : ''}${m.close ? ' close' : ''}`);
        await context.close();
      }
    }
  }
  const meta = { commit: gitCommit(), sourceHash: SOURCE_HASH, browser: browser.version(), platform: `${os.platform()} ${os.release()}`,
    still: flag('--still'), guides: flag('--guides'), taken: new Date().toISOString(), shots };
  fs.writeFileSync(path.join(dir, 'metrics.json'), JSON.stringify(meta, null, 1));
  writeGallery();
}

// A fixed touch drag on the phone: 60 moves of 6px left, then 60 back, each a frame apart.
async function perf(browser, run = 1) {
  const config = CONFIGS[0];
  const { context, page } = await openArcade(browser, config, `?room=${MODE}`);
  const cdp = await context.newCDPSession(page);
  await cdp.send('Performance.enable');
  await page.waitForTimeout(9500);       // the arrows' nudge has finished
  await page.waitForFunction(() => [...document.querySelectorAll('#game-grid .card[data-loop]')].every((c) => c.hasAttribute('data-loop-ready')));
  if (flag('--warm-screens')) {
    const count = await page.locator('#game-grid .card').count();
    for (const key of ['ArrowRight', 'ArrowLeft']) for (let i = 0; i < count; i++) {
      await page.keyboard.press(key);
      await page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'), null, { timeout: 15000 });
    }
    await page.waitForFunction(() => [...document.querySelectorAll('#game-grid .card')].every((card) => card.hasAttribute('data-loop-ready')));
  }
  const read = async () => Object.fromEntries((await cdp.send('Performance.getMetrics')).metrics.map((m) => [m.name, m.value]));
  if (flag('--trace')) await startTrace(cdp);
  await page.evaluate(() => {
    window.__frames = [];
    let last = 0;
    (function tick(now) { if (last) window.__frames.push(now - last); last = now; requestAnimationFrame(tick); })(performance.now());
  });
  const point = (x) => [{ x, y: 400, id: 1 }];
  const before = await read();
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: point(300) });
  let x = 300, moves = 0;
  for (const dir of [-1, 1]) {
    for (let i = 0; i < 60; i++) {
      x += dir * 6; moves++;
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: point(x) });
      await page.evaluate(() => new Promise(requestAnimationFrame));
    }
  }
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const after = await read();
  const frames = await page.evaluate(() => window.__frames.slice());
  frames.sort((a, b) => a - b);
  const per = (name) => (after[name] - before[name]) / moves;
  const result = {
    commit: gitCommit(), sourceHash: SOURCE_HASH, run, browser: browser.version(), platform: `${os.platform()} ${os.release()}`, mode: MODE,
    hardware: await hardware(browser), screensWarmed: flag('--warm-screens'),
    viewport: `${config.width}×${config.height}`, moves,
    layoutsPerMove: per('LayoutCount'), styleRecalcsPerMove: per('RecalcStyleCount'),
    layoutMsPerMove: per('LayoutDuration') * 1000, styleMsPerMove: per('RecalcStyleDuration') * 1000,
    scriptMsPerMove: per('ScriptDuration') * 1000,
    frameMs: { median: frames[frames.length >> 1], p95: frames[Math.floor(frames.length * 0.95)], count: frames.length },
    taken: new Date().toISOString(),
  };
  fs.mkdirSync(OUT, { recursive: true });
  const name = `perf-${MODE}-${result.commit.replace(/\W/g, '_')}-run-${run}`;
  if (flag('--trace')) result.trace = await finishTrace(cdp, name);
  const file = path.join(OUT, `${name}.json`);
  fs.writeFileSync(file, JSON.stringify(result, null, 1));
  console.log(JSON.stringify(result, null, 1));
  console.log(`wrote ${path.relative(ROOT, file)}`);
  await context.close();
}

async function hardware(browser) {
  const session = await browser.newBrowserCDPSession();
  try {
    const info = await session.send('SystemInfo.getInfo');
    return { cpu: [...new Set(os.cpus().map((c) => c.model))], cpuCount: os.cpus().length,
      renderer: info.gpu.auxAttributes?.glRenderer || '(not reported)', vendor: info.gpu.auxAttributes?.glVendor,
      devices: info.gpu.devices.map((device) => device.deviceString), features: info.gpu.featureStatus };
  } finally { await session.detach(); }
}

async function startTrace(cdp) {
  await cdp.send('Tracing.start', { categories: 'devtools.timeline,disabled-by-default-devtools.timeline.frame,blink.user_timing', transferMode: 'ReturnAsStream' });
}

async function finishTrace(cdp, name) {
  const complete = new Promise((resolve) => cdp.once('Tracing.tracingComplete', resolve));
  await cdp.send('Tracing.end');
  const { stream } = await complete, chunks = [];
  try {
    for (;;) {
      const chunk = await cdp.send('IO.read', { handle: stream, size: 65536 });
      chunks.push(Buffer.from(chunk.data, chunk.base64Encoded ? 'base64' : 'utf8'));
      if (chunk.eof) break;
    }
  } finally { await cdp.send('IO.close', { handle: stream }); }
  const data = Buffer.concat(chunks), events = JSON.parse(data).traceEvents || [];
  fs.mkdirSync(OUT, { recursive: true });
  const file = `${name}.trace.json.gz`; fs.writeFileSync(path.join(OUT, file), gzipSync(data));
  const count = (name) => events.filter((event) => event.name === name && event.ph === 'X').length;
  return { file, events: events.length, layouts: count('Layout'), styles: count('UpdateLayoutTree') };
}

function frameStats(frames) {
  const sorted = frames.slice().sort((a, b) => a - b);
  return { count: frames.length, median: sorted[sorted.length >> 1], p95: sorted[Math.floor(sorted.length * .95)],
    over50ms: frames.filter((n) => n > 50).length, over250ms: frames.filter((n) => n > 250).length };
}

async function soak(browser) {
  const { context, page } = await openArcade(browser, CONFIGS[3], `?room=${MODE}`);
  const cdp = await context.newCDPSession(page);
  await cdp.send('Performance.enable'); await cdp.send('HeapProfiler.enable');
  await page.waitForFunction(() => [...document.querySelectorAll('.card[data-loop]')].every((card) => card.hasAttribute('data-loop-ready')));
  await page.waitForTimeout(9500);
  const result = { commit: gitCommit(), sourceHash: SOURCE_HASH, browser: browser.version(), hardware: await hardware(browser),
    platform: `${os.platform()} ${os.release()}`, mode: MODE, viewport: '1080×1920', minutes: SOAK_MINUTES,
    gc: 'HeapProfiler.collectGarbage twice at minute 1 and the final minute', samples: [], started: new Date().toISOString() };
  const name = `soak-${MODE}-${result.commit.replace(/\W/g, '_')}-${SOAK_MINUTES}min`;
  fs.mkdirSync(OUT, { recursive: true });
  const file = path.join(OUT, `${name}.json`), start = Date.now();
  for (let minute = 1; minute <= SOAK_MINUTES; minute++) {
    while (Date.now() < start + minute * 60000) await new Promise((done) => setTimeout(done, Math.min(1000, start + minute * 60000 - Date.now())));
    const gc = minute === 1 || minute === SOAK_MINUTES;
    if (gc) { await cdp.send('HeapProfiler.collectGarbage'); await cdp.send('HeapProfiler.collectGarbage'); }
    const metrics = Object.fromEntries((await cdp.send('Performance.getMetrics')).metrics.map((m) => [m.name, m.value]));
    const dom = await cdp.send('Memory.getDOMCounters');
    const visibility = await page.evaluate(() => document.visibilityState);
    result.samples.push({ minute, elapsedMs: Date.now() - start, gc, heapBytes: metrics.JSHeapUsedSize, dom, visibility });
    fs.writeFileSync(file, JSON.stringify(result, null, 1));
    console.log(`soak ${minute}/${SOAK_MINUTES}: heap=${metrics.JSHeapUsedSize} nodes=${dom.nodes} visibility=${visibility}${gc ? ' after GC' : ''}`);
  }
  const first = result.samples[0], last = result.samples.at(-1);
  result.retainedGrowth = (last.heapBytes - first.heapBytes) / first.heapBytes;
  result.withinHeapTarget = result.retainedGrowth <= .1;
  await startTrace(cdp);
  await page.evaluate(() => {
    window.__soakFrames = [];
    let last = performance.now();
    const end = last + 30000;
    function frame(now) { window.__soakFrames.push(now - last); last = now; if (now < end) requestAnimationFrame(frame); }
    requestAnimationFrame(frame);
  });
  await page.waitForTimeout(30500);
  result.frames = frameStats(await page.evaluate(() => window.__soakFrames));
  result.trace = await finishTrace(cdp, name);
  result.finished = new Date().toISOString();
  fs.writeFileSync(file, JSON.stringify(result, null, 1));
  console.log(JSON.stringify({ file: path.relative(ROOT, file), retainedGrowth: result.retainedGrowth, frames: result.frames }, null, 1));
  if (!result.withinHeapTarget || result.samples.some((sample) => sample.visibility !== 'visible')) process.exitCode = 1;
  await context.close();
}

function writeGallery() {
  const modes = ['full', 'lite'].map((mode) => {
    const file = path.join(OUT, mode, 'metrics.json');
    return fs.existsSync(file) ? [mode, JSON.parse(fs.readFileSync(file, 'utf8'))] : null;
  }).filter(Boolean);
  const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
  const crops = (shot) => shot.layers.filter((l) => l.shown && l.visible).map((l) => {
    const v = l.visible, scale = Math.min(1, 220 / v.width);
    return `<figure class="crop"><div style="width:${(v.width * scale).toFixed(0)}px;height:${(Math.min(v.height, 300) * scale).toFixed(0)}px;` +
      `background:url('${esc(shot.file)}') -${(v.x * scale).toFixed(0)}px -${(v.y * scale).toFixed(0)}px/` +
      `${(Number(shot.viewport.split('×')[0]) * scale).toFixed(0)}px auto"></div><figcaption>${esc(l.layer)}</figcaption></figure>`;
  }).join('');
  const sections = modes.map(([mode, meta]) => `
  <h2>${mode} <small>${esc(meta.commit)} · ${esc(meta.browser)} · ${meta.still ? 'still' : 'live'}${meta.guides ? ' · guides' : ''} · ${esc(meta.taken)}</small></h2>
  <div class="grid">${meta.shots.map((s) => `
    <article>
      <a href="${esc(s.file)}"><img src="${esc(s.file)}" alt="${esc(s.file)}" loading="lazy"></a>
      <p><b>${esc(s.config)}</b> ${esc(s.viewport)} · ${esc(s.position)} · ${s.credits ? 'one credit' : 'no credit'}<br>
      in view: ${Object.entries(s.seen || {}).map(([k, v]) => `${esc(k)} ${v}%`).join(' · ')}<br>
      ${esc(s.cabinet)} @ ${s.aislePos} · layout ${esc(s.layout)}${s.fullscreen ? ' · kiosk' : ''}${s.close ? ' · close' : ''} · room ${esc(s.room)}</p>
      <div class="crops">${crops(s)}</div>
    </article>`).join('')}
  </div>`).join('');
  const html = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Arcade room captures</title>
<style>
  body {margin:0;padding:16px;background:#16120f;color:#f1e6d8;font:14px/1.45 system-ui,sans-serif}
  h1 {font-size:20px;margin:0 0 8px} h2 {font-size:16px;margin:28px 0 10px} small {color:#b9a58f;font-weight:normal}
  .refs,.grid {display:grid;gap:14px;grid-template-columns:repeat(auto-fill,minmax(260px,1fr))}
  .refs img {width:100%;height:220px;object-fit:cover;border-radius:6px}
  article {background:#241d18;border-radius:8px;padding:8px} article img {width:100%;border-radius:4px;display:block}
  article p {margin:6px 0;font-size:12px;color:#d9c9b6}
  .crops {display:flex;flex-wrap:wrap;gap:6px} .crop {margin:0} .crop div {border:1px solid #5a4a3c;background-repeat:no-repeat}
  .crop figcaption {font-size:11px;color:#b9a58f}
  ul {margin:6px 0 0;padding-left:20px}
</style></head><body>
<h1>Arcade room captures (issue #65)</h1>
<p>Compare each capture with the photographs at actual size. These pictures are evidence for a visual judgement, not a score.</p>
<ul>${CHECKLIST.map((c) => `<li>${esc(c)}</li>`).join('')}</ul>
<h2>References</h2>
<div class="refs">${REFERENCES.map(([label, url]) => `<figure style="margin:0"><a href="${url}"><img src="${url}" alt="${esc(label)}"></a><figcaption>${esc(label)}</figcaption></figure>`).join('')}</div>
${sections}
</body></html>
`;
  fs.writeFileSync(path.join(OUT, 'index.html'), html);
  console.log(`wrote ${path.relative(ROOT, path.join(OUT, 'index.html'))}`);
}

const server = await listenOn(createFileServer({ overrides: baselineSources }), PORT);
const browser = await chromium.launch({ executablePath: chromiumBinary(), headless: true, args: ['--mute-audio'] });
try {
  if (flag('--soak')) await soak(browser);
  else if (flag('--perf')) for (let run = 1; run <= RUNS; run++) await perf(browser, run);
  else await capture(browser);
  if (sourceHash() !== SOURCE_HASH) throw new Error('Room sources changed during verification; rerun against a stable revision');
} finally {
  await browser.close();
  server.close();
}
