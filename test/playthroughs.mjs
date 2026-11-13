// Every game's bot plays through the themes of its game: one test per
// playthrough, named in the game's bot.rs (`PLAYTHROUGHS`), run one at a time.
//
//   node --test test/playthroughs.mjs               (all games)
//   BLIP_PLAYTHROUGH_GAME=serpent node --test test/playthroughs.mjs

import test from 'node:test';
import assert from 'node:assert/strict';
import { GAME_SLUGS } from './lib/games.mjs';
import { buildGames, listPlaythroughs, playBot } from './lib/native-bot.mjs';

const only = process.env.BLIP_PLAYTHROUGH_GAME;
const games = only ? only.split(',') : GAME_SLUGS;
assert.ok(buildGames(games), 'the release build failed');

for (const game of games) {
  const names = listPlaythroughs(game);
  test(`${game} lists its playthroughs`, () => {
    assert.ok(names.length > 0, `${game}'s bot.rs has no PLAYTHROUGHS`);
  });
  for (const name of names) {
    test(`${game}: ${name}`, () => {
      // Brawler's ladder runs take about 500 s on their own: room for a busy machine.
      const r = playBot(game, { BLIP_BOT_PLAYTHROUGH: name }, 900);
      assert.equal(r.error, undefined, r.error);
      assert.equal(r.playthrough, 'pass', `missing ${r.missing} after ${r.t}s (ended: ${r.outcome})`);
    });
  }
}
