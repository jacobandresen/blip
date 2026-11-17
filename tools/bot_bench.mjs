// How well a game's bot plays, over several seeds: the mean of every RESULT stat, so a change to a bot (or to
// blip::bot) can be measured, not guessed. Runs one game at a time, one seed after another.
//
//   node tools/bot_bench.mjs <game> [--seeds 1-6] [--build] [BLIP_BOT_X=value ...]
//   node tools/bot_bench.mjs raider --seeds 1-4 BLIP_BOT_MAXT=90 BLIP_BOT_LEVEL=3
//   node tools/bot_bench.mjs rounds "BLIP_BOT_PLAYTHROUGH=it rides the whole week"
//
// Needs cargo and, on Linux without a display, xvfb-run (see test/lib/native-bot.mjs).

import { buildGames, playBot } from '../test/lib/native-bot.mjs';

const args = process.argv.slice(2);
const game = args.shift();
if (!game) throw new Error('usage: node tools/bot_bench.mjs <game> [--seeds a-b] [--build] [BLIP_BOT_X=v ...]');
let seeds = [1, 2, 3, 4];
const env = {};
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--seeds') {
    const [a, b] = args[++i].split('-').map(Number);
    seeds = Array.from({ length: (b ?? a) - a + 1 }, (_, k) => a + k);
  } else if (args[i] === '--build') {
    if (!buildGames([game])) process.exit(1);
  } else {
    const eq = args[i].indexOf('=');
    env[args[i].slice(0, eq)] = args[i].slice(eq + 1);
  }
}

const sums = {}, runs = [];
for (const seed of seeds) {
  const r = playBot(game, { ...env, BLIP_BOT_SEED: String(seed) });
  runs.push(r);
  console.log(`seed ${seed}: ${r.error ?? Object.entries(r).map(([k, v]) => `${k}=${v}`).join(' ')}`);
  for (const [k, v] of Object.entries(r)) if (!Number.isNaN(Number(v))) sums[k] = (sums[k] ?? 0) + Number(v);
}
const ok = runs.filter((r) => !r.error).length;
const passes = runs.filter((r) => r.playthrough === 'pass').length;
console.log(`mean over ${ok} runs${env.BLIP_BOT_PLAYTHROUGH ? `, ${passes}/${runs.length} passed` : ''}: ` +
  Object.entries(sums).filter(([k]) => !k.startsWith('t_level')).map(([k, v]) => `${k}=${+(v / ok).toFixed(2)}`).join(' '));
