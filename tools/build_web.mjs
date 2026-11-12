// Build every game for the web: each game's wasm to web/<game>/index.wasm, and the
// game page (web/game.html) copied beside it as index.html.
//
//   node tools/build_web.mjs        (npm run build:web)
//
// Needs Rust with the wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown).
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { GAME_SLUGS } from '../test/lib/games.mjs';
import { runInherited } from './lib/process.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
// rustup's cargo, where rustup is installed: a distribution's own cargo may lack the wasm target.
const rustup = (tool) => { try { return execFileSync('rustup', ['which', tool], { encoding: 'utf8' }).trim(); } catch { return null; } };
const cargo = rustup('cargo') || 'cargo', rustc = rustup('rustc');

const args = ['build', '--release', '--target', 'wasm32-unknown-unknown', ...GAME_SLUGS.flatMap((game) => ['-p', game])];
console.log(`[build] cargo ${args.join(' ')}`);
await runInherited(cargo, args, { cwd: root, env: rustc ? { ...process.env, RUSTC: rustc } : process.env });

for (const game of GAME_SLUGS) {
  const out = path.join(root, 'web', game);
  mkdirSync(out, { recursive: true });
  copyFileSync(path.join(root, 'target/wasm32-unknown-unknown/release', `${game}.wasm`), path.join(out, 'index.wasm'));
  copyFileSync(path.join(root, 'web/game.html'), path.join(out, 'index.html'));
  console.log(`[ok] ${game} -> web/${game}/index.wasm (${statSync(path.join(out, 'index.wasm')).size} bytes)`);
}
