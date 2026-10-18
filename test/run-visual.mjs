import { spawn } from 'node:child_process';

const games = process.env.BLIP_VISUAL_GAME
  ? [process.env.BLIP_VISUAL_GAME]
  : ['ADDER', 'BOUNCER', 'BRAWLER', 'BUBBLER', 'DEFENDER', 'METEORS', 'RALLY', 'SERPENT', 'SKY_RAIDER'];

for (const game of games) {
  const code = await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, ['--test', 'test/visual-parity.mjs'], {
      cwd: new URL('../', import.meta.url),
      env: { ...process.env, BLIP_VISUAL_GAME: game },
      stdio: 'inherit',
    });
    child.once('error', reject);
    child.once('close', (status) => resolve(status ?? 1));
  });
  if (code !== 0) process.exit(code);
}
