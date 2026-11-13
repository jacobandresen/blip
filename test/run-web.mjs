// The browser suites in two lanes at once. The tests that need a running game sample its frames, so they get a lane
// to themselves; the ones that only read the shell's layout share the other.
//
//   node test/run-web.mjs

import { run } from './lib/run.mjs';

const LANES = [
  { name: 'game', concurrency: 1, files: ['touch', 'controls', 'two-player-phones', 'two-player-join'] },
  { name: 'layout', concurrency: 2, files: ['manual', 'cabinet', 'machine-identity', 'machine-launch', 'attract', 'iphone14-room', 'kiosk-room', 'touch-controls', 'aisle-perf', 'arcade-room', 'two-player-deck', 'topbar-layout', 'home'] },
];

const codes = await Promise.all(LANES.map((lane) => run(process.execPath,
  ['--test', `--test-concurrency=${lane.concurrency}`, ...lane.files.map((f) => `test/${f}.mjs`)], { heading: `${lane.name} lane` })));
// exitCode, not exit(): a piped stdout must drain the last lane's report first
process.exitCode = codes.some((code) => code !== 0) ? 1 : 0;
