// Estimates how much fun each game is from its playtest bot, on three measures in order of importance (the reasons
// and sources are in crates/blip/src/bot/FUN.md):
//
//   1. skill  (weight 3): the bot's score against random play's (BLIP_BOT_FUZZ); 100 times or more is 1.
//   2. close  (weight 2): near misses / (near misses + hits), from hit boxes; only games whose bot reports them.
//   3. busy   (weight 1): 1 - the share of time with nothing to do (crates/blip/src/bot/fun.rs).
//
// fun = 100 x the weighted mean of the measures a game has, per seed; a game's fun is the mean over seeds.
// Needs cargo; on Linux, xvfb-run or a desktop display. One game runs at a time.
//
//   node tools/fun_report.mjs [game ...] [--seeds 1,2,3,4,5,6] [--secs 60] [--root <checkout>] [--json <file>]
//                             [--near <dir>] [--against <json> [--tolerance 5]]
//   node tools/fun_report.mjs --changed <base>
//   node tools/fun_report.mjs --from <json> [--against <json>]   (score saved runs again, without playing)
//
// --changed prints, as JSON, the games whose code differs from the commit <base>: a game's own crate, or every game
// when shared code (crates/blip, Cargo.*, the toolchain, this tool, the game list) changed.
// --root measures another checkout (built there). --near saves seed 1's near misses and hits with their hit boxes.
// --against compares with an earlier --json, seed by seed, and exits 1 when a game is less fun: when its fun score, or
// the raw signal under one of its measures, gets worse by more than a set amount and by more than twice the change's
// standard error (so chance alone rarely fails a run). The raw signals are checked as well as the score because a
// weighted score dilutes them: a bug that tripled a game's hits once cost its score only 3.6 points.
// CI runs it on each pull request's base and head (.github/workflows/fun.yml).

import { spawnSync } from 'node:child_process';
import { appendFileSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { GAME_SLUGS } from '../test/lib/games.mjs';
import { displayCommand } from './lib/process.mjs';

const argv = process.argv.slice(2);
const opt = (name, fallback) => { const i = argv.indexOf(name); return i < 0 ? fallback : argv.splice(i, 2)[1]; };
const root = path.resolve(opt('--root', fileURLToPath(new URL('../', import.meta.url))));
const seeds = opt('--seeds', '1,2,3,4,5,6').split(',').map(Number);
const secs = opt('--secs', '60');
const json = opt('--json', null);
const near = opt('--near', null);
const against = opt('--against', null);
const tolerance = Number(opt('--tolerance', '5'));
const changedSince = opt('--changed', null);
if (changedSince) {
  // Docs and pictures change no play.
  const files = spawnSync('git', ['diff', '--name-only', `${changedSince}...HEAD`], { encoding: 'utf8' }).stdout.split('\n')
    .filter((f) => f && !/\.(md|png|webp|svg)$/.test(f));
  const shared = files.some((f) => /^(crates\/blip\/|Cargo\.|rust-toolchain|tools\/fun_report\.mjs|tools\/lib\/|test\/lib\/games\.mjs|web\/blip_core\.js)/.test(f));
  console.log(JSON.stringify(shared ? GAME_SLUGS : GAME_SLUGS.filter((g) => files.some((f) => f.startsWith(`crates/${g}/`)))));
  process.exit(0);
}
const from = opt('--from', null);
const saved = from ? JSON.parse(readFileSync(from, 'utf8')).report : null;
const games = argv.length ? argv : saved ? Object.keys(saved) : GAME_SLUGS;
const WEIGHTS = { skill: 3, close: 2, busy: 1 };

if (!saved) {
  const build = spawnSync('cargo', ['build', '--release', ...games.flatMap((g) => ['-p', g])], { cwd: root, stdio: 'inherit' });
  if (build.status !== 0) process.exit(1);
}

/** One run's RESULT stats (numbers where they are numbers), or { error }. */
function run(game, env) {
  const [command, args] = displayCommand(path.join(root, 'target/release', game), [], { cwd: root, screen: '1024x900x24' });
  const r = spawnSync(command, args, {
    cwd: root, encoding: 'utf8', timeout: 900_000,
    env: { ...process.env, BLIP_BOT: '1', BLIP_SILENT: '1', BLIP_CRT: '0', BLIP_BOT_MAXT: secs, ...env },
  });
  const line = `${r.stdout}`.split('\n').find((l) => l.startsWith('RESULT '));
  if (!line) return { error: `no RESULT (exit ${r.status})` };
  return Object.fromEntries(line.slice(7).split(' ').map((kv) => {
    const [k, v] = [kv.slice(0, kv.indexOf('=')), kv.slice(kv.indexOf('=') + 1)];
    return [k, Number.isNaN(Number(v)) ? v : Number(v)];
  }));
}

const clamp01 = (v) => Math.max(0, Math.min(1, v));
const mean = (xs) => xs.reduce((a, b) => a + b, 0) / Math.max(1, xs.length);
const sd = (xs) => xs.length < 2 ? 0 : Math.sqrt(xs.reduce((a, b) => a + (b - mean(xs)) ** 2, 0) / (xs.length - 1));
const round = (v, d = 1) => Math.round(v * 10 ** d) / 10 ** d;

/** One seed's measures: its bot run against random play over all seeds (random play is a baseline, not a pair). */
function measures(bot, randomScore) {
  const m = { skill: clamp01(Math.log10(((bot.score ?? 0) + 1) / (randomScore + 1)) / 2) };
  const passes = (bot.fun_near_pm ?? 0) + (bot.fun_hits_pm ?? 0);
  if (bot.fun_near_pm !== undefined && passes > 0) m.close = bot.fun_near_pm / passes;
  m.busy = 1 - (bot.fun_dead ?? 0);
  return m;
}

const fun = (m) => 100 * Object.entries(m).reduce((a, [k, v]) => a + WEIGHTS[k] * v, 0)
  / Object.keys(m).reduce((a, k) => a + WEIGHTS[k], 0);

const report = {};
for (const game of games) {
  const playedSeeds = saved ? saved[game].runs.map((r) => r.seed) : seeds;
  const bots = saved ? saved[game].runs.map((r) => r.bot)
    : seeds.map((s, i) => run(game, { BLIP_BOT_SEED: String(s), ...(near && i === 0 ? { BLIP_BOT_NEAR: mkdir(path.join(near, game)) } : {}) }));
  const randoms = saved ? saved[game].runs.map((r) => r.random).filter(Boolean)
    : seeds.map((s) => run(game, { BLIP_BOT_SEED: String(s), BLIP_BOT_FUZZ: String(s) }));
  const randomScore = mean(randoms.filter((r) => !r.error).map((r) => r.score ?? 0));
  const perSeed = bots.map((b, i) => (b.error ? null : { seed: playedSeeds[i], m: measures(b, randomScore) }));
  const ok = perSeed.filter(Boolean);
  const funs = ok.map((p) => fun(p.m));
  const avg = (k) => { const v = ok.map((p) => p.m[k]).filter((x) => x !== undefined); return v.length ? round(mean(v), 2) : undefined; };
  report[game] = {
    fun: round(mean(funs)), sd: round(sd(funs)),
    skill: avg('skill'), close: avg('close'), busy: avg('busy'),
    seeds: Object.fromEntries(ok.map((p, i) => [p.seed, round(funs[i])])),
    errors: bots.length - ok.length + randoms.filter((r) => r.error).length,
    runs: bots.map((bot, i) => {
      const k = ok.findIndex((p) => p.seed === playedSeeds[i]);
      return { seed: playedSeeds[i], fun: k < 0 ? undefined : round(funs[k]), measures: k < 0 ? undefined : ok[k].m, bot, random: randoms[i] };
    }),
  };
  console.error(game, report[game].fun, '±', report[game].sd);
}

function mkdir(dir) { mkdirSync(dir, { recursive: true }); return dir; }

/** Seed-by-seed change from `was` to `now` (runs paired by seed), of `value(run, other)`: its mean and standard error. */
function change(was, now, value) {
  const d = now.runs.map((r) => [r, was.runs.find((w) => w.seed === r.seed)])
    .filter(([r, w]) => w && !r.bot.error && !w.bot.error)
    .map(([r, w]) => value(r, w) - value(w, r)).filter((x) => Number.isFinite(x));
  return d.length >= 2 ? { mean: mean(d), se: sd(d) / Math.sqrt(d.length) } : undefined;
}

/** A run's fun on only the measures the run it is compared with also has: a measure one side cannot take (a game
 * that gains a hit-box hook) must not count as a change in fun. */
function sharedFun(run, other) {
  if (!run.measures || !other.measures) return run.fun;
  return fun(Object.fromEntries(Object.entries(run.measures).filter(([k]) => k in other.measures)));
}

/** What gets worse, by how much counts, and how a run gives it; `worse` is the sign of a loss. */
const CHECKS = [
  { name: 'fun', limit: tolerance, worse: -1, value: sharedFun },
  { name: 'score (log ratio)', limit: Math.log10(1 / 0.9), worse: -1, value: (r) => Math.log10((r.bot.score ?? 0) + 1) },
  { name: 'hits a minute', limit: 1, worse: 1, value: (r) => r.bot.fun_hits_pm },
  { name: 'dead time', limit: 0.05, worse: 1, value: (r) => r.bot.fun_dead },
];

const before = against ? JSON.parse(readFileSync(against, 'utf8')).report : null;
const falls = [];
const lines = [`| game | fun |${before ? ' before | change |' : ''} skill | close | busy |`, `|---|---|${before ? '---|---|' : ''}---|---|---|`];
for (const [game, r] of Object.entries(report)) {
  const c = before?.[game] ? change(before[game], r, sharedFun) : undefined;
  if (before?.[game]) {
    for (const k of CHECKS) {
      const ch = change(before[game], r, k.value);
      if (ch && ch.mean * k.worse > k.limit && ch.mean * k.worse > 2 * ch.se) {
        falls.push(`${game}: ${k.name} ${ch.mean > 0 ? '+' : ''}${round(ch.mean, 2)} ± ${round(ch.se, 2)}`);
      }
    }
  }
  const shown = c ? ` ${before[game].fun} | ${c.mean >= 0 ? '+' : ''}${round(c.mean)} ± ${round(c.se)} |` : before ? ' | |' : '';
  lines.push(`| ${game} | ${r.fun} ± ${r.sd} |${shown} ${r.skill ?? ''} | ${r.close ?? ''} | ${r.busy ?? ''} |`);
}
const table = lines.join('\n');
console.log(table);
if (json) writeFileSync(json, JSON.stringify({ secs, seeds, report }, null, 1));
if (process.env.GITHUB_STEP_SUMMARY) {
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### Fun estimate (${seeds.length} seeds, ${secs} s each)\n\n${table}\n\n`
    + (falls.length ? `**Less fun:** ${falls.join('; ')}\n` : ''));
}
if (falls.length) {
  console.error(`Less fun than before: ${falls.join('; ')} (worse by more than its limit and twice its standard error)`);
  process.exit(1);
}
