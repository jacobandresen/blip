import { spawn } from 'node:child_process';

const gameArg = process.argv.indexOf('--game');
const selectedGame = gameArg >= 0 ? process.argv[gameArg + 1] : process.env.BLIP_VISUAL_GAME;
const artifacts = process.argv.includes('--artifacts') || process.env.BLIP_VISUAL_ARTIFACTS === '1';
const games = selectedGame
  ? [selectedGame]
  : ['ADDER', 'BOUNCER', 'BRAWLER', 'BUBBLER', 'DEFENDER', 'METEORS', 'RALLY', 'SERPENT', 'RAIDER'];

const failures = [];
for (const game of games) {
  const code = await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, ['--test', 'test/visual-parity.mjs'], {
      cwd: new URL('../', import.meta.url),
      env: { ...process.env, BLIP_VISUAL_GAME: game, BLIP_VISUAL_ARTIFACTS: artifacts ? '1' : '0' },
      stdio: 'inherit',
    });
    child.once('error', reject);
    child.once('close', (status) => resolve(status ?? 1));
  });
  if (code !== 0) failures.push(game);
}
if (failures.length) console.error(`Visual parity failed: ${failures.join(', ')}`);
process.exitCode = failures.length ? 1 : 0;
