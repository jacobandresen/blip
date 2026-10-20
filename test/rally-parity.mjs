import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { openPage } from './lib/harness.mjs';

const root = new URL('../', import.meta.url);
const source = await readFile(new URL('web/TAPE/RALLY.BLIP', root), 'utf8');
const fields = ['BX', 'BY', 'VX', 'VY', 'BALL_SPEED', 'PLAYER_SCORE', 'CPU_SCORE', 'PY', 'CY'];
function fixture(scenario, steps) {
  const lines = new Map(source.trim().split('\n').map((line) => {
    const at = line.indexOf(' '); return [Number(line.slice(0, at)), line.slice(at + 1)];
  }));
  lines.set(10, 'MODE=2:GOTO 800');
  lines.set(200, `DT=${scenario === 'fast' ? '.1' : '.016'}:GOSUB 1100:GOTO 500`);
  let setup = 'GOSUB 1500:STATE=1:STOP_MUSIC:VX=250:VY=80:LPAD_VY=0:RPAD_VY=0';
  if (scenario === 'top') setup += ':BY=29:VY=0-200';
  if (scenario === 'bottom') setup += ':BY=527:VY=200';
  if (['left', 'edge', 'spin', 'cap', 'fast', 'miss', 'corner'].includes(scenario)) setup += ':BX=41:VX=0-275:VY=0';
  if (scenario === 'edge') setup += ':BY=245.5';
  if (scenario === 'spin') setup += ':LPAD_VY=300';
  if (scenario === 'cap') setup += ':VX=0-445:BALL_SPEED=445';
  if (scenario === 'fast') setup += ':BX=65:VX=0-450:BALL_SPEED=450';
  if (scenario === 'miss') setup += ':BY=400';
  if (scenario === 'corner') setup += ':PY=30:BY=27:VY=0-80';
  if (['right', 'right-spin'].includes(scenario)) setup += ':BX=427:VX=275:VY=0';
  if (scenario === 'right-spin') setup += ':RPAD_VY=0-300';
  if (scenario === 'point-left') setup += ':BX=0-11:BY=400:VX=0-275:VY=0';
  if (scenario === 'point-right') setup += ':BX=479:BY=400:VX=275:VY=0';
  lines.set(810, setup + ':GOTO 200');
  lines.set(1400, 'RETURN'); // Compare the ball at the instant the native scorer receives the point.
  lines.set(589, `STEPS=STEPS+1:IF STEPS>=${steps} THEN GOTO 3000`);
  for (const [i, field] of fields.entries()) lines.set(3000 + i * 10, `SOUND 1000+${field},0,0:WAIT 0`);
  lines.set(4000, 'WAIT 0:GOTO 4000');
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
