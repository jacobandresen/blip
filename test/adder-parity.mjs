import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openPage } from './lib/harness.mjs';

const root = new URL('../', import.meta.url);
const source = await readFile(new URL('web/TAPE/ADDER.BLIP', root), 'utf8');
const fields = [
  'ROUND', 'ROUND_WAIT', 'WORLD_OVER',
  ...[1, 2].flatMap((s) => [`HEAD_X(${s})`, `HEAD_Y(${s})`, `ANGLE(${s})`, `LENGTH(${s})`, `ALIVE(${s})`, `LIFE(${s})`, `PTS(${s})`, `HUMAN(${s})`, `HOLES(${s})`]),
];

// The tape at a fixed step, its vipers placed where the Rust example puts
// them, steering by a fixed trace, and the fields reported as tones.
function fixture(scenario, steps) {
  const lines = new Map(source.trim().split('\n').map((line) => {
    const separator = line.indexOf(' '); return [Number(line.slice(0, separator)), line.slice(separator + 1)];
  }));
  const cpu = ['cpuwall', 'cpu-gaps'].includes(scenario);
  let start = 'START_GAME 1:STEPS=0';
  if (scenario === 'join') start += ':JOIN_PLAYER_TWO';
  if (scenario === 'wall' || scenario === 'over') start += ':HEAD_X(1)=627';
  if (scenario === 'over') start += ':LIFE(1)=1';
  lines.set(85, start);
  lines.set(111, 'DT=0.016');
  for (const n of lines.keys()) if (n > 2710 && n < 2820) lines.delete(n);
  lines.set(2720, 'HEAD_X(S)=100+(S-1)*260:HEAD_Y(S)=180+(S-1)*200:ANGLE(S)=(S-1)*3.141592653589793');
  if (scenario === 'cpuwall') lines.set(2730, 'IF S=2 THEN HEAD_X(2)=600:ANGLE(2)=0');
  // An extra sample at (102,180), laid first so it is the trail's oldest.
  if (['body', 'gap', 'spent', 'overlap'].includes(scenario)) {
    const gap = scenario === 'body' ? 0 : 1;
    if (gap) lines.set(2740, `IF S=2 THEN HOLE_COUNT=1:HOLE_OWNER(1)=2:HOLE_SPENT(1)=${scenario === 'spent' ? 1 : 0}:HOLE_FIRST(1)=1:HOLES(2)=1`);
    lines.set(2750, `IF S=2 THEN ADD_SAMPLE 2,102,180,${gap}`);
    if (scenario === 'overlap') lines.set(2760, 'IF S=2 THEN ADD_SAMPLE 2,102,180,0');
  }
  const turn = scenario === 'left' ? -1 : scenario === 'right' ? 1 : 0;
  lines.set(5020, cpu ? 'IF S=1 THEN TURN(1)=0 ELSE STEER_CPU S' : `IF S=1 THEN TURN(1)=${turn} ELSE TURN(2)=${scenario === 'join' ? -1 : 0}`);
  lines.set(165, `STEPS=STEPS+1:IF STEPS>=${steps} OR WORLD_OVER THEN GOTO 9900`);
  for (const [i, field] of fields.entries()) lines.set(9900 + i * 4, `SOUND 1000+${field},0,0:WAIT 1`);
  lines.set(9999, 'WAIT 16:GOTO 9999');
  return [...lines];
}

function unpack(values) {
  return {
    round: Math.round(values[0]), round_wait: values[1], over: !!values[2],
    snakes: [0, 1].map((i) => {
      const [x, y, angle, length, alive, lives, score, human, gaps] = values.slice(3 + i * 9, 12 + i * 9);
      return { x, y, angle, length: Math.round(length), alive: !!alive, lives: Math.round(lives), score: Math.round(score), human: !!human, gaps: Math.round(gaps) };
    }),
  };
}

test('Adder BASIC follows native movement, trail, collision, score, gap, and life rules', async (t) => {
  execFileSync('cargo', ['build', '--release', '-p', 'adder', '--example', 'adder_parity'], { cwd: root, stdio: 'pipe' });
  const { page, origin } = await openPage(t, 'chromium');
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){(window.__traceRows ||= []).push(f-1000);};');
    await route.fulfill({ response, body });
  });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto(`${origin}/basic.html`);
  await page.evaluate(() => BlipBasic.submit('LOAD /TAPE/ADDER'));
  // Not 250 CPU steps: then seat 1 ends exactly on a sample boundary, where
  // Rust's f32 positions and BASIC's f64 ones round to different counts.
  const cases = [['straight', 60], ['left', 60], ['right', 60], ['join', 60], ['cpuwall', 60], ['cpu-gaps', 251], ['straight', 190], ['wall', 1], ['body', 1], ['gap', 1], ['spent', 1], ['overlap', 1], ['over', 1]]
    .filter(([scenario]) => !process.env.BLIP_PARITY_CASE || scenario === process.env.BLIP_PARITY_CASE);
  for (const [scenario, steps] of cases) await t.test(`${scenario}, ${steps} steps`, async () => {
    const frame = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
    if (frame) await frame.evaluate(() => { window.__traceRows = []; });
    await page.evaluate((lines) => {
      BlipBasic.lines.clear();
      for (const [n, code] of lines) BlipBasic.lines.set(n, code);
      BlipBasic.submit('RUN');
    }, fixture(scenario, steps));
    await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
    const runtime = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
    try {
      await runtime.waitForFunction((count) => window.__traceRows?.length >= count, fields.length, { timeout: 20000 });
    } catch {
      assert.fail(`Trace stopped: ${await page.locator('body').getAttribute('data-wasm-error')}; rows ${JSON.stringify(await runtime.evaluate(() => window.__traceRows))}`);
    }
    const actual = unpack(await runtime.evaluate(() => window.__traceRows));
    const expected = JSON.parse(execFileSync(fileURLToPath(new URL('target/release/examples/adder_parity', root)), [scenario, String(steps)], { encoding: 'utf8' }));
    assert.equal(actual.round, expected.round);
    assert.equal(actual.over, expected.over);
    assert.ok(Math.abs(actual.round_wait - expected.round_wait) < .001, `round wait: BASIC ${JSON.stringify(actual)}, Rust ${JSON.stringify(expected)}`);
    for (const [i, snake] of actual.snakes.entries()) {
      for (const field of ['x', 'y', 'angle']) assert.ok(Math.abs(snake[field] - expected.snakes[i][field]) < .003,
        `seat ${i + 1} ${field}: BASIC ${snake[field]}, Rust ${expected.snakes[i][field]}`);
      for (const field of ['length', 'alive', 'lives', 'score', 'human', 'gaps']) assert.equal(snake[field], expected.snakes[i][field], `seat ${i + 1} ${field}: BASIC ${JSON.stringify(actual)}, Rust ${JSON.stringify(expected)}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  });
});
