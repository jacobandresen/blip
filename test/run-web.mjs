// The browser suites in two lanes at once. The tests that need a running game sample its frames (and aisle-perf
// times them), so they get a lane to themselves; the ones that only read the shell's layout share the other, two at a
// time (three starved the game lane on four cores), the longest first so none starts last (measured alone
// 2026-11-26: iphone14-room 105 s, manual 101 s, cabinet 95 s, attract and two-player-deck 52 s; the rest under 40 s).
//
//   node test/run-web.mjs

import { run } from './lib/run.mjs';

const LANES = [
  { name: 'game', concurrency: 1, files: ['touch', 'two-player-phones', 'aisle-perf', 'two-player-join', 'controls'] },
  { name: 'layout', concurrency: 2, files: ['iphone14-room', 'manual', 'cabinet', 'attract', 'two-player-deck', 'home',
    'arcade-room', 'touch-controls', 'machine-identity', 'topbar-layout', 'kiosk-room', 'machine-launch'] },
];

const codes = await Promise.all(LANES.map((lane) => run(process.execPath,
  ['--test', `--test-concurrency=${lane.concurrency}`, ...lane.files.map((f) => `test/${f}.mjs`)], { heading: `${lane.name} lane` })));
// exitCode, not exit(): a piped stdout must drain the last lane's report first
process.exitCode = codes.some((code) => code !== 0) ? 1 : 0;
