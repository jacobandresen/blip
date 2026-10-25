// Photograph matching poses through Rust's game renderer and BASIC's interpreter.
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { chromiumBinary } from '../test/lib/chromium-binary.mjs';
import { createFileServer, listenOn } from '../test/lib/harness.mjs';
import { numberedTape } from '../test/lib/basic-tape.mjs';
import { displayCommand } from './lib/process.mjs';

const exec = promisify(execFile);
const root = new URL('../', import.meta.url);
const output = new URL('../test/artifacts/brawler-compare/', import.meta.url);
const tape = await readFile(new URL('../web/TAPE/BRAWLER.BLIP', import.meta.url), 'utf8');
const atlas = JSON.parse(await readFile(new URL('../crates/brawler/art/atlas.json', import.meta.url), 'utf8'));
const scenes = [
  ['ryuka-stance', 0, 0], ['ryuka-walk', 0, 1], ['ryuka-punch', 0, 7],
  ['ryuka-kick', 0, 10], ['ryuka-jump', 0, 12], ['ryuka-uppercut', 0, 15],
  ['jade-kick', 10, 10], ['jade-alt-kick', 10, 10, 1, 1], ['jade-guard', 10, 3], ['regent-special', 11, 11],
  ['regent-knockdown', 11, 5], ['kite-special', 12, 11],
  ['aurelia-special', 13, 11], ['aurelia-victory', 13, 6], ['choose-jade', 10, 'SELECT'],
  ['choose-webber', 7, 'SELECT'],
  ['choose-versus', 14, 'SELECT', 10, 0, 'VERSUS'],
  ['rooftop-film', 0, 0, 7],
  ['choose-zenith', 9, 'SELECT'],
  ['mack-stance', 14, 0], ['mack-special', 14, 11], ['choose-mack', 14, 'SELECT'],
  ['ryuka-walk-far', 0, 17], ['regent-rise', 11, 16], ['title', 0, 'TITLE'],
];
const only = process.argv.indexOf('--scene');
const selected = only >= 0 ? scenes.filter(([name]) => name === process.argv[only + 1]) : scenes;
if (!selected.length) throw new Error('Unknown comparison scene');
const acts = ['IDLE', 'WALK', 'CROUCH', 'BLOCK', 'HITSTUN', 'KNOCKDOWN', 'VICTORY',
  'ATTACK', 'ATTACK', 'ATTACK', 'ATTACK', 'ATTACK', 'AIR', 'ATTACK', 'ATTACK', 'ATTACK', 'KNOCKDOWN', 'WALK'];
const moves = [0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 0, 6, 7, 8, 0, 0];
const frameArg = process.argv.indexOf('--frame');
const frameOverride = frameArg >= 0;
const frames = Number(frameOverride ? process.argv[frameArg + 1] : process.env.BLIP_COMPARE_FRAME || 3);
if (!Number.isInteger(frames) || frames < 0 || frames > 5) throw new Error('--frame must be between 0 and 5');
const tickArg = process.argv.indexOf('--tick');
const tickOverride = tickArg >= 0;
const compareTick = tickOverride ? Number(process.argv[tickArg + 1])
  : process.env.BLIP_COMPARE_TICK === undefined ? null : Number(process.env.BLIP_COMPARE_TICK);
if (compareTick !== null && (!Number.isInteger(compareTick) || compareTick < 0 || compareTick > 183)) {
  throw new Error('--tick must be between 0 and 183');
}
const screenshotArg = process.argv.indexOf('--screenshot-frame');
const screenshotFrame = Number(screenshotArg >= 0 ? process.argv[screenshotArg + 1] : 2);
if (!Number.isInteger(screenshotFrame) || screenshotFrame < 1 || screenshotFrame > 5) {
  throw new Error('--screenshot-frame must be between 1 and 5');
}
const clockArg = process.argv.indexOf('--clock');
const compareClock = Number(clockArg >= 0 ? process.argv[clockArg + 1] : process.env.BLIP_COMPARE_CLOCK || 32);
if (!Number.isInteger(compareClock) || compareClock < 0 || compareClock > 45) {
  throw new Error('--clock must be an integer between 0 and 45');
}
const data = tape.split('\n').filter((line) => /^61\d{3} DATA /.test(line))
  .map((line) => line.split(' DATA ')[1].split(',').map((v) => Number(v)));
const roster = tape.split('\n').filter((line) => /^62\d{3} DATA /.test(line))
  .map((line) => line.split(' DATA ')[1].split(',').map((v) => v.trim().replace(/^"|"$/g, '')));

function basicScene(who, pose, foe, altKick, compareMode) {
  const source = numberedTape(tape);
  let inside = false;
  for (const [n, line] of [...source]) {
    if (line.trim() === 'SUB UPDATE_PLAY') { inside = true; continue; }
    if (inside && line.trim() === 'END SUB') { inside = false; continue; }
    if (inside) source.delete(n);
  }
  if (pose === 'TITLE') {
    source.set(90, 'STATE$="TITLE"');
    for (const [n, line] of source) if (line === 'NOW=TIME()') source.set(n, 'NOW=2');
  } else if (pose === 'SELECT') {
    source.set(90, `${compareMode === 'VERSUS' ? 'MODE=2:' : ''}PICK=${who + 1}:PICK2=${foe + 1}:STATE$="SELECT"`);
  } else {
    const move = moves[pose], m = data[move - 1];
    let ticks = frames * 4;
    if (move) ticks = [m[1] * .15, m[1] * .55, m[1] * .9, m[1] + m[2] * .5,
      m[1] + m[2] + m[3] * .3, m[1] + m[2] + m[3] * .7][frames];
    if (pose === 5) ticks = [0, 3, 6, 10, 14, 19][frames];
    if (pose === 16) ticks = 51 + frames * 3;
    if (pose === 11 && who === 14 && compareTick !== null) ticks = compareTick;
    const x = pose === 1 || pose === 17 ? (216 + (frames + (pose === 17 ? 6 : 0)) * 6) * atlas[who].size : 240;
    const y = pose === 12 || pose === 13 ? 260 : 330;
    const home = Number(roster[who === 14 ? who : foe][9]);
    const setup = [`PICK=${who + 1}`, 'START_GAME', `WHO(2)=${foe + 1}`, `STAGE=${home}`,
      `CLOCK=${compareClock}`, 'INTRO=0', `X(1)=${x}`, 'X(2)=400', `Y(1)=${y}`,
      `ACT$(1)="${acts[pose]}"`, `MV(1)=${move}`, `T(1)=${ticks}`,
      ...(altKick ? ['ALT_KICK(1)=1'] : []),
      `VY(1)=${(-500 + frames * 200) / 60}`, 'HEALTH(2)=MAX_HP(2)', 'GHOST(2)=HEALTH(2)'];
    if (pose === 11 && who === 14 && compareTick !== null) {
      const shotTicks = [56, 71, 86];
      const shots = shotTicks.filter((at) => at <= ticks).length;
      const stopX = Math.max(90, Math.min(550, x - 130));
      setup.push(`DRIVEBY_ACTIVE(1)=1:DRIVEBY_T(1)=${ticks}:DRIVEBY_START_X(1)=${x}:DRIVEBY_FACING(1)=1:DRIVEBY_SHOTS(1)=${shots}`);
      for (let slot = 1; slot <= 3; slot++) {
        const at = shotTicks[slot - 1];
        if (ticks < at) continue;
        const carX = stopX;
        const boltX = carX + 39 + (ticks - at) * (520 / 60);
        const bounce = Math.sin((at / 60 - 23 / 60) * 10.5) * 6.5;
        const dy = [-5, 0, 5][slot - 1];
        if (boltX >= 0 && boltX <= 640) setup.push(`SHOT(1,${slot})=1:SHOT_X(1,${slot})=${boltX}:SHOT_Y(1,${slot})=${273 + bounce + dy}:SHOT_FACE(1,${slot})=1:SHOT_SPEED(1,${slot})=520/60:SHOT_AT(1,${slot})=${at}`);
      }
    }
    source.set(90, setup.join(':'));
    const now = pose === 0
      ? (frames + .01) * Math.PI * 2 / (atlas[who].rate * 6)
      : 2;
    for (const [n, line] of source) if (line === 'NOW=TIME()') source.set(n, `NOW=${now}`);
  }
  return [...source].sort(([a], [b]) => a - b).map(([n, line]) => `${n} ${line}`).join('\n');
}

await mkdir(output, { recursive: true });
const server = createFileServer();
await listenOn(server, 0);
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
const report = [];

async function captureRust(env) {
  const options = { cwd: fileURLToPath(root), timeout: 60000, env };
  const binary = fileURLToPath(new URL('../target/release/brawler', import.meta.url));
  const [command, args] = displayCommand(binary, [], { cwd: options.cwd });
  return exec(command, args, options);
}

try {
  browser = await chromium.launch({ executablePath: chromiumBinary(), headless: true, args: ['--mute-audio'] });
  for (const [name, who, pose, foe = 1, altKick = 0, compareMode = 'SOLO'] of selected) {
    const frameSuffix = `${frameOverride ? `-f${frames}` : ''}${tickOverride ? `-t${compareTick}` : ''}${screenshotFrame === 2 ? '' : `-s${screenshotFrame}`}`;
    const rustFile = new URL(`${name}${frameSuffix}-rust.png`, output);
    await captureRust({ ...process.env, BLIP_SILENT: '1', BLIP_COMPARE: String(pose), BLIP_COMPARE_MODE: compareMode, BLIP_COMPARE_CLOCK: String(compareClock), BLIP_COMPARE_WHO: String(who), BLIP_COMPARE_FOE: String(foe),
        ...(altKick ? { BLIP_COMPARE_ALT_KICK: '1' } : {}),
        BLIP_COMPARE_FRAME: String(frames), BLIP_SCREENSHOT_FRAME: String(screenshotFrame),
        ...(compareTick === null ? {} : { BLIP_COMPARE_TICK: String(compareTick) }),
        BLIP_SCREENSHOT_OUT: fileURLToPath(rustFile), BLIP_CRT: '0' });
    const page = await browser.newPage({ viewport: { width: 640, height: 400 }, deviceScaleFactor: 1 });
    try {
      await page.addInitScript(() => {
        window.BlipBasic = { isFileAvailable: (path) => path.startsWith('/TAPE/BRAWLER/ASSETS/') };
        window.addEventListener('message', (event) => {
          if (event.data?.type === 'blip-basic-program-loaded') window.__compareReady = true;
          if (event.data?.type === 'blip-basic-runtime-error') window.__compareError = event.data.message;
        });
      });
      await page.goto(`${origin}/basic-runtime.html`);
      await page.evaluate((source) => { window.blipBasicSource = source; }, basicScene(who, pose, foe, altKick, compareMode));
      await page.waitForFunction(() => window.__compareReady || window.__compareError, undefined, { timeout: 30000 });
      const error = await page.evaluate(() => window.__compareError);
      if (error) throw new Error(`${name}: ${error}`);
      // Let the first WebGL frame reach Chromium's screenshot compositor.
      await page.waitForTimeout(500);
      const basicFile = new URL(`${name}${frameSuffix}-basic.png`, output);
      await page.locator('#glcanvas').screenshot({ path: fileURLToPath(basicFile) });
      const encoded = await Promise.all([rustFile, basicFile].map(async (file) => (await readFile(file)).toString('base64')));
      report.push(`<section><h2>${name}</h2><div class="pair"><figure><figcaption>Rust</figcaption><img src="data:image/png;base64,${encoded[0]}"></figure><figure><figcaption>BASIC</figcaption><img src="data:image/png;base64,${encoded[1]}"></figure></div></section>`);
      console.log(`Compared ${name}`);
    } finally { await page.close(); }
  }
  const html = `<!doctype html><meta charset="utf-8"><title>Brawler: Rust / BASIC</title><style>
    body{background:#15131d;color:#f8edda;font:16px system-ui;margin:24px}h1{font-size:24px}.pair{display:flex;gap:16px;flex-wrap:wrap}figure{margin:0}img{width:var(--size,640px);max-width:100%;image-rendering:pixelated}figcaption{margin-bottom:8px}section{margin:32px 0}button{padding:10px;margin-right:8px}
    </style><h1>Brawler: matching poses in Rust and BASIC</h1><p>Clean 640 × 400 captures. Stage animations and HUD presentation differ; compare body proportions, contact, poses and legibility.</p><button onclick="document.documentElement.style.setProperty('--size','640px')">640px</button><button onclick="document.documentElement.style.setProperty('--size','390px')">Phone size</button>${report.join('')}`;
  await writeFile(new URL('index.html', output), html);
  console.log(fileURLToPath(new URL('index.html', output)));
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
