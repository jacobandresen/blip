import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openPage } from './lib/harness.mjs';

const root = new URL('../', import.meta.url);
const source = await readFile(new URL('web/TAPE/RALLY.BLIP', root), 'utf8');
const fields = ['BALL_X', 'BALL_Y', 'BALL_VX', 'BALL_VY', 'BALL_SPEED', 'LEFT_SCORE', 'RIGHT_SCORE', 'LPAD_Y', 'RPAD_Y'];
// The tape with its main loop cut down to MOVE_BALL at a fixed step, its
// scorer reduced to the count, and the fields reported as tones.
function fixture(scenario, steps) {
  const lines = new Map(source.trim().split('\n').map((line) => {
    const at = line.indexOf(' '); return [Number(line.slice(0, at)), line.slice(at + 1)];
  }));
  let setup = 'PLAYERS=2:START_GAME:STATE$="PLAY":BALL_VX=250:BALL_VY=80:LPAD_VY=0:RPAD_VY=0';
  if (scenario === 'top') setup += ':BALL_Y=33:BALL_VY=-200';
  if (scenario === 'bottom') setup += ':BALL_Y=387:BALL_VY=200';
  if (['left', 'edge', 'spin', 'cap', 'fast', 'miss', 'corner'].includes(scenario)) setup += ':BALL_X=45:BALL_VX=-360:BALL_VY=0';
  if (scenario === 'edge') setup += ':BALL_Y=181.5';
  if (scenario === 'spin') setup += ':LPAD_VY=300';
  if (scenario === 'cap') setup += ':BALL_VX=-590:BALL_SPEED=590';
  if (scenario === 'fast') setup += ':BALL_X=85:BALL_VX=-600:BALL_SPEED=600';
  if (scenario === 'miss') setup += ':BALL_Y=330';
  if (scenario === 'corner') setup += ':LPAD_Y=34:BALL_Y=31:BALL_VY=-80';
  if (['right', 'right-spin'].includes(scenario)) setup += ':BALL_X=583:BALL_VX=360:BALL_VY=0';
  if (scenario === 'right-spin') setup += ':RPAD_VY=-300';
  if (scenario === 'point-left') setup += ':BALL_X=-11:BALL_Y=330:BALL_VX=-360:BALL_VY=0';
  if (scenario === 'point-right') setup += ':BALL_X=639:BALL_Y=330:BALL_VX=360:BALL_VY=0';
  lines.set(80, setup);
  lines.set(110, `DT=${scenario === 'fast' ? '.1' : '.016'}`);
  for (let n = 111; n <= 160; n++) lines.delete(n);
  lines.set(130, 'MOVE_BALL');
  lines.set(165, `STEPS=STEPS+1:IF STEPS>=${steps} THEN GOTO 9900`);
  for (let n = 7020; n <= 7180; n += 10) lines.delete(n);
  lines.set(7020, 'IF LEFT_SCORED THEN LEFT_SCORE=LEFT_SCORE+1 ELSE RIGHT_SCORE=RIGHT_SCORE+1');
  for (const [i, field] of fields.entries()) lines.set(9900 + i * 10, `SOUND 1000+${field},0,0:WAIT 0`);
  lines.set(9995, 'WAIT 0:GOTO 9995');
  return [...lines];
}

test('Rally BASIC matches native swept collisions, spin, speed, walls, and points', async (t) => {
  execFileSync('cargo', ['build', '--release', '-p', 'rally', '--example', 'rally_parity'], { cwd: root, stdio: 'pipe' });
  const { page, origin } = await openPage(t, 'chromium');
  await page.route('**/basic-runtime.html*', async (route) => {
    const response = await route.fetch();
    await route.fulfill({ response, body: (await response.text()).replace('imports.env.blip_basic_sound=playBasicTone;',
      'imports.env.blip_basic_sound=function(f,s,v){(window.__trace ||= []).push(f-1000);};') });
  });
  await page.goto(`${origin}/basic.html`);
  for (const [scenario, steps] of [['flight', 30], ...['top', 'bottom', 'left', 'edge', 'spin', 'cap', 'fast', 'miss', 'corner', 'right', 'right-spin', 'point-left', 'point-right'].map((name) => [name, 1])]) {
    await t.test(`${scenario}, ${steps} steps`, async () => {
      const old = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
      if (old) await old.evaluate(() => { window.__trace = []; });
      await page.evaluate((lines) => {
        BlipBasic.lines.clear();
        for (const [n, code] of lines) BlipBasic.lines.set(n, code);
        BlipBasic.submit('RUN');
      }, fixture(scenario, steps));
      await page.waitForFunction(() => document.body.hasAttribute('data-wasm-running') || document.body.hasAttribute('data-wasm-error'));
      const runtime = page.frames().find((frame) => frame.url().includes('basic-runtime.html'));
      try { await runtime.waitForFunction(() => window.__trace?.length >= 9, undefined, { timeout: 10000 }); }
      catch { assert.fail(`Rally trace stopped: ${await page.locator('body').getAttribute('data-wasm-error')}`); }
      const actual = await runtime.evaluate(() => window.__trace);
      const expected = JSON.parse(execFileSync(fileURLToPath(new URL('target/release/examples/rally_parity', root)), [scenario, String(steps)], { encoding: 'utf8' }));
      for (const [i, field] of fields.entries()) assert.ok(Math.abs(actual[i] - expected[i]) < .003, `${field}: BASIC ${actual[i]}, Rust ${expected[i]}`);
      assert.equal(await page.locator('body').getAttribute('data-wasm-error'), null);
    });
  }
});
