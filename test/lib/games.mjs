// The games the site offers, read from their one registry (BLIP_GAMES in
// web/blip_core.js), so suites never keep their own count or list.

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';

// No browser plumbing here, so unit suites can import it too.
const WEB_DIR = fileURLToPath(new URL('../../web/', import.meta.url));

// Only the registry's object literals are evaluated: the rest of blip_core.js needs a page.
const SOURCE = readFileSync(path.join(WEB_DIR, 'blip_core.js'), 'utf8');
function literal(name) {
  const start = SOURCE.indexOf('{', SOURCE.indexOf(`var ${name} =`));
  let depth = 0, quote = null;
  for (let i = start; i < SOURCE.length; i++) {
    const c = SOURCE[i];
    if (quote) { if (c === '\\') i++; else if (c === quote) quote = null; continue; }
    if (c === '/' && SOURCE[i + 1] === '/') i = SOURCE.indexOf('\n', i);
    else if (c === '"' || c === "'" || c === '`') quote = c;
    else if (c === '{') depth++;
    else if (c === '}' && --depth === 0) return SOURCE.slice(start, i + 1);
  }
  throw new Error(`${name} not found in blip_core.js`);
}
function registry() {
  return vm.runInNewContext(`var TWO_PLAYER_KEYS = ${literal('TWO_PLAYER_KEYS')}; (${literal('BLIP_GAMES')})`);
}

/** Every game, { slug, name, ... } in the registry's order. */
export const GAMES = Object.entries(registry()).map(([slug, game]) => ({ ...game, slug }));
export const GAME_SLUGS = GAMES.map((game) => game.slug);
/** The arcade aisle's machines, left to right, as web/index.html lays out their cards. */
export const AISLE = [...readFileSync(path.join(WEB_DIR, 'index.html'), 'utf8').matchAll(/data-game-url="([a-z]+)\//g)].map((m) => m[1]);
