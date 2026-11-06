// Plays every level of every game with its native autopilot, one run at a time.
//
//   node tools/playtest_levels.mjs [game ...] [--seconds 30] [--fuzz <seed>]
//
// Builds the release binaries, then for each game starts a bot run at each
// level (BLIP_BOT_LEVEL=n) for --seconds of game time. A game with authored
// levels reports how many (the `levels` stat); an endless game is sampled at
// a few levels. A run fails if it panics, prints no RESULT line, or does not
// start on the level asked for. Also prints the level each run reached; raise
// --seconds to check that the bot clears the slower levels.
// Needs cargo; on Linux, xvfb-run or a desktop display.

import { GAME_SLUGS } from '../test/lib/games.mjs';
import { buildGames, playBot } from '../test/lib/native-bot.mjs';

const ENDLESS = [1, 2, 3, 5, 8, 12];

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const i = args.indexOf(name);
  if (i < 0) return fallback;
  const [value] = args.splice(i, 2).slice(1);
  return value;
};
const seconds = Number(option('--seconds', 30));
const fuzz = option('--fuzz', null);
const games = args.length ? args : GAME_SLUGS;
for (const game of games) if (!GAME_SLUGS.includes(game)) throw new Error(`no game called ${game}`);

if (!buildGames(games)) process.exit(1);

/** One bot run from `level`: its RESULT stats, or the reason it has none. */
function play(game, level) {
  return playBot(game, { BLIP_BOT_MAXT: String(seconds), BLIP_BOT_LEVEL: String(level), ...(fuzz ? { BLIP_BOT_FUZZ: fuzz } : {}) }, seconds * 4 + 60);
}

let failed = 0;
for (const game of games) {
  const first = play(game, 1);
  const count = Number(first.levels) || 0;
  const levels = first.error || first.level === undefined ? [1] : count ? Array.from({ length: count }, (_, i) => i + 1) : ENDLESS;
  for (const level of levels) {
    const r = level === 1 ? first : play(game, level);
    let verdict;
    if (r.error) verdict = `FAIL ${r.error}`;
    else if (r.level !== undefined && Number(r.level) < level) verdict = `FAIL started on level ${r.level}`;
    else if (r.level === undefined) verdict = `${r.outcome} (no levels)`;
    else verdict = `${r.outcome}, reached ${r.level}${Number(r.level) === level && r.outcome === 'timeout' ? ` (not cleared in ${seconds}s)` : ''}`;
    if (verdict.startsWith('FAIL')) failed++;
    console.log(`${game.padEnd(9)} level ${String(level).padStart(2)}${count ? `/${count}` : ''}: ${verdict}`);
  }
}
if (failed) {
  console.error(`${failed} run(s) failed`);
  process.exit(1);
}
