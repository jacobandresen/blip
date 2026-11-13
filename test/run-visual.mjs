import { run } from './lib/run.mjs';
import { GAME_SLUGS } from './lib/games.mjs';

const gameArg = process.argv.indexOf('--game');
const selectedGame = gameArg >= 0 ? process.argv[gameArg + 1] : process.env.BLIP_VISUAL_GAME;
const artifacts = process.argv.includes('--artifacts') || process.env.BLIP_VISUAL_ARTIFACTS === '1';
const games = selectedGame
  ? [selectedGame]
  : GAME_SLUGS.map((slug) => slug.toUpperCase());

const failures = [];
for (const game of games) {
  const code = await run(process.execPath, ['--test', 'test/visual.mjs'],
    { env: { ...process.env, BLIP_VISUAL_GAME: game, BLIP_VISUAL_ARTIFACTS: artifacts ? '1' : '0' } });
  if (code !== 0) failures.push(game);
}
if (failures.length) console.error(`Visual check failed: ${failures.join(', ')}`);
process.exitCode = failures.length ? 1 : 0;
