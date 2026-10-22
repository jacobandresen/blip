// Each machine's attract loop on its arcade screen: a few seconds of the native autopilot's play, as one strip.
//
//   node tools/make_attract_strips.mjs [game ...]     (after a game's look changes)
//
// Runs each game's autopilot in turn (never two at once) under xvfb-run, keeps FRAMES frames from
// FROM seconds on, and writes web/<slug>/attract.webp: the frames stacked top to bottom. The arcade
// plays FRAMES of them (its card-loop animation) and reads their shape from the image.
// Generated: edit this file, not its output.

import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, rmSync, statSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const GAMES = ['rally', 'bouncer', 'defender', 'bubbler', 'raider', 'meteors', 'serpent', 'brawler', 'adder'];
const FRAMES = 12, EVERY = 0.25, WIDTH = 240, LIMIT = 60 * 1024;
// Where the loop starts, past each game's title screen and into play.
const FROM = { rally: 6, bouncer: 6, defender: 8, bubbler: 8, raider: 8, meteors: 3, serpent: 6, brawler: 10, adder: 8 };

const only = process.argv.slice(2);

for (const slug of GAMES.filter((g) => !only.length || only.includes(g))) {
  const bin = path.join(ROOT, 'target/release', slug);
  if (!existsSync(bin)) throw new Error(`${bin} is missing: cargo build --release -p ${slug}`);
  const shots = mkdtempSync(path.join(tmpdir(), `blip-attract-${slug}-`));
  try {
    const from = FROM[slug];
    try {
      execFileSync('xvfb-run', ['-a', '-s', '-screen 0 1024x900x24', bin], {
        stdio: ['ignore', 'ignore', 'inherit'], timeout: 120000,
        // Bubbler plays with an autopilot of its own; BLIP_BOT still takes the shots.
        env: { ...process.env, BLIP_BOT: '1', BLIP_CRT: '0', BLIP_BOT_SHOTS: shots, BLIP_BOT_SHOT_EVERY: String(EVERY), BLIP_BOT_MAXT: String(from + FRAMES * EVERY + 2),
          ...(slug === 'bubbler' ? { BUBBLER_BOT: '1' } : {}) },
      });
    } catch (error) {
      if (error.code === 'ETIMEDOUT') throw error; // the autopilot ends its run with a status of its own
    }
    // Shots are taken from 1 s on, one every EVERY seconds.
    const first = Math.round((from - 1) / EVERY) + 1;
    const names = readdirSync(shots).filter((f) => f.endsWith('.png')).sort();
    const picked = names.slice(first - 1, first - 1 + FRAMES).map((f) => path.join(shots, f));
    if (picked.length < FRAMES) throw new Error(`${slug}: only ${names.length} frames`);
    const [w, h] = execFileSync('magick', ['identify', '-format', '%w %h', picked[0]]).toString().split(' ').map(Number);
    const out = path.join(ROOT, 'web', slug, 'attract.webp');
    // The busiest games (stars, scrolling scenery) take a lower quality, then a narrower strip, to stay under the limit.
    let quality = 75, width = WIDTH, size;
    for (;;) {
      execFileSync('magick', [...picked, '-filter', 'Box', '-resize', `${width}x`, '-append', '-quality', String(quality), '-define', 'webp:method=6', out]);
      size = statSync(out).size;
      if (size <= LIMIT) break;
      if (quality > 50) quality -= 5;
      else if (width > 160) width -= 20;
      else throw new Error(`${slug}: ${size} bytes at ${width}px and quality ${quality}, over ${LIMIT}`);
    }
    console.log(`${slug}: ${FRAMES} frames of ${w}x${h} from ${from} s, ${width}px at quality ${quality}, ${(size / 1024).toFixed(1)} KB`);
  } finally {
    rmSync(shots, { recursive: true, force: true });
  }
}
