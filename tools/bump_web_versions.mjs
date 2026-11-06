// Bump the cache versions for what changed under web/ since the last commit:
// the `?v=` of every changed file where a page or script links it, and the
// service worker's CACHE once. Running it again bumps nothing new.
//
//   node tools/bump_web_versions.mjs
//
// Needs git. Per-game pages are copied from web/shell.html, so run
// `cargo build-web` afterwards to carry shell.html's versions to them.

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const web = path.join(root, 'web');
const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' });
const committed = (file) => { try { return git('show', `HEAD:${file}`); } catch { return ''; } };

const changed = git('status', '--porcelain', '--', 'web').split('\n').filter(Boolean)
  .map((line) => line.slice(3).split(' -> ').pop())
  .filter((file) => !file.endsWith('/sw.js'));
if (!changed.length) { console.log('nothing under web/ has changed'); process.exit(0); }

// The pages and scripts that link assets; per-game pages are generated, so they are left to cargo build-web.
const linkers = readdirSync(web).filter((name) => /\.(html|js|css)$/.test(name)).map((name) => path.join('web', name));
const names = [...new Set(changed.map((file) => path.basename(file)))];

let bumped = 0;
for (const file of linkers) {
  const full = path.join(root, file);
  const before = committed(file), now = readFileSync(full, 'utf8');
  let text = now;
  for (const name of names) {
    const pattern = new RegExp(`(${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\?v=)(\\d+)`, 'g');
    text = text.replace(pattern, (whole, prefix, version, offset) => {
      // Bump a link only while it still carries its committed version.
      const old = before.match(new RegExp(`${prefix.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}(\\d+)`));
      if (old && old[1] !== version) return whole;
      bumped++;
      return prefix + (Number(version) + 1);
    });
  }
  if (text !== now) { writeFileSync(full, text); console.log(`bumped links in ${file}`); }
}

const sw = path.join(web, 'sw.js'), swText = readFileSync(sw, 'utf8');
const cache = /var CACHE = "blip-v(\d+)";/;
const [, current] = swText.match(cache), [, last] = committed('web/sw.js').match(cache) || [, current];
if (current === last) {
  writeFileSync(sw, swText.replace(cache, `var CACHE = "blip-v${Number(current) + 1}";`));
  console.log(`service worker cache blip-v${Number(current) + 1}`);
}
console.log(`${bumped} links bumped for ${names.length} changed files; run cargo build-web to copy shell.html to the game pages`);
