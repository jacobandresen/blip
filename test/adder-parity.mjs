import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openPage } from './lib/harness.mjs';

const root = new URL('../', import.meta.url);
const source = await readFile(new URL('web/TAPE/ADDER.BLIP', root), 'utf8');
const fields = [
  'ROUND', 'ROUND_WAIT', '(GAME_STATE=2)',
  ...[1, 2].flatMap((s) => [`HEAD_X(${s})`, `HEAD_Y(${s})`, `ANGLE(${s})`, `LENGTH(${s})`, `ALIVE(${s})`, `LIFE(${s})`, `PTS(${s})`, `HUMAN(${s})`, `TRACE_GAPS(${s})`]),
];

function fixture(scenario, steps) {
  const lines = new Map(source.trim().split('\n').map((line) => {
    const separator = line.indexOf(' '); return [Number(line.slice(0, separator)), line.slice(separator + 1)];
  }));
  lines.set(10, 'MODE=1:GOTO 800');
  lines.set(100, 'DT=0.016');
  lines.set(120, 'REM JOIN IS FIXED BY THE SCENARIO');
  lines.set(130, `TURN(1)=${scenario === 'left' ? -1 : scenario === 'right' ? 1 : 0}`);
  lines.set(140, `TURN(2)=${scenario === 'join' ? -1 : 0}`);
  if (!['cpuwall', 'cpu-gaps'].includes(scenario)) lines.set(160, 'REM BOTH HEADINGS USE THE FIXED INPUT TRACE');
  lines.set(589, `STEPS=STEPS+1:IF STEPS>=${steps} THEN GOTO 3000`);
  lines.set(700, 'GAME_STATE=2:GOTO 3000');
  lines.set(830, 'STOP_MUSIC:GOSUB 1100:GOTO 100');
  for (const n of lines.keys()) if (n >= 1120 && n <= 1166) lines.delete(n);
  lines.set(1120, 'BEST_X=100+(S-1)*260:BEST_Y=180+(S-1)*200:ANGLE(S)=(S-1)*3.141592653589793:HEAD_X(S)=BEST_X:HEAD_Y(S)=BEST_Y');
  if (scenario === 'cpuwall') lines.set(1121, 'IF S=2 THEN BEST_X=440:HEAD_X(S)=440:ANGLE(S)=0');
  lines.set(1180, 'NEXT S:STEPS=0');
  if (scenario === 'join') lines.set(1181, 'MODE=2:HUMAN(2)=1:LIFE(2)=3');
  if (scenario === 'wall' || scenario === 'over') lines.set(1181, 'HEAD_X(1)=467');
  if (scenario === 'over') lines.set(1182, 'LIFE(1)=1');
  if (['body', 'gap', 'spent', 'overlap'].includes(scenario)) {
    lines.set(1181, 'S=2:SAMPLE_X=102:SAMPLE_Y=180:SAMPLE_GAP=0');
    if (scenario !== 'body') lines.set(1182, `GAP_COUNT=1:GAP_OWNER(1)=2:GAP_SPENT(1)=${scenario === 'spent' ? 1 : 0}:SAMPLE_GAP=1`);
    lines.set(1183, 'GOSUB 1200');
    if (scenario === 'overlap') lines.set(1184, 'SAMPLE_GAP=0:GOSUB 1200');
  }
  lines.set(1189, 'RETURN');
  lines.set(3000, 'TRACE_GAPS(1)=0:TRACE_GAPS(2)=0:FOR G=1 TO GAP_COUNT');
  lines.set(3001, 'IF GAP_COUNT>0 THEN TRACE_GAPS(GAP_OWNER(G))=TRACE_GAPS(GAP_OWNER(G))+1');
  lines.set(3002, 'NEXT G');
  for (const [i, field] of fields.entries()) lines.set(3100 + i * 10, `SOUND 1000+${field},0,0:WAIT 1`);
  lines.set(4000, 'WAIT 16:GOTO 4000');
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
  const cases = [['straight', 60], ['left', 60], ['right', 60], ['join', 60], ['cpuwall', 60], ['cpu-gaps', 250], ['straight', 190], ['wall', 1], ['body', 1], ['gap', 1], ['spent', 1], ['overlap', 1], ['over', 1]]
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
      for (const field of ['length', 'alive', 'lives', 'score', 'human', 'gaps']) assert.equal(snake[field], expected.snakes[i][field], `seat ${i + 1} ${field}`);
    }
    assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
  });
});
