// Runs a game's native autopilot (crates/blip/src/bot) and reads its RESULT line.
// Needs cargo; on Linux, xvfb-run or a desktop display.

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { displayCommand } from '../../tools/lib/process.mjs';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));

/** Build the release binaries of `games`; false if the build failed. */
export function buildGames(games) {
  const build = spawnSync('cargo', ['build', '--release', ...games.flatMap((g) => ['-p', g])], { cwd: ROOT, stdio: 'inherit' });
  return build.status === 0;
}

function launch(game, env, timeoutSecs) {
  const [command, args] = displayCommand(path.join(ROOT, 'target/release', game), [], { cwd: ROOT, screen: '1024x900x24' });
  const run = spawnSync(command, args, {
    cwd: ROOT, encoding: 'utf8', timeout: timeoutSecs * 1000,
    env: { ...process.env, BLIP_BOT: '1', BLIP_SILENT: '1', BLIP_CRT: '0', ...env },
  });
  return { run, out: `${run.stdout}\n${run.stderr}` };
}

/** The game's playthrough names (`blip::bot::playthroughs`). */
export function listPlaythroughs(game) {
  const { out } = launch(game, { BLIP_BOT_LIST: '1' }, 60);
  return out.split('\n').filter((l) => l.startsWith('PLAYTHROUGH ')).map((l) => l.slice(12));
}

/**
 * One bot run with these BLIP_BOT_* settings: its RESULT stats as strings
 * ({ outcome, t, level, playthrough, missing, ... }), or { error }.
 */
export function playBot(game, env, timeoutSecs = 600) {
  const { run, out } = launch(game, env, timeoutSecs);
  const panic = out.split('\n').find((l) => l.includes('panicked'));
  if (panic) return { error: panic.trim() };
  if (run.error) return { error: run.error.message };
  const line = out.split('\n').find((l) => l.startsWith('RESULT '));
  if (!line) return { error: `no RESULT line (exit ${run.status})` };
  return Object.fromEntries(line.slice(7).split(' ').map((kv) => [kv.slice(0, kv.indexOf('=')), kv.slice(kv.indexOf('=') + 1)]));
}
