// The browser suites in two lanes at once. The tests that need a running game sample its frames, so they get a lane
// to themselves; the ones that only read the shell's layout share the other.
//
//   node test/run-web.mjs

import { spawn } from 'node:child_process';

const LANES = [
  { name: 'game', concurrency: 1, files: ['touch', 'controls', 'two-player-touch-phones', 'bubbler-two-player-iphone', 'basic-controls', 'basic-game-controls', 'basic-canvas', 'basic-tape', 'basic-mobile-layout', 'brawler-basic', 'adder-controls', 'adder-parity', 'rally-parity'] },
  { name: 'layout', concurrency: 2, files: ['manual', 'cabinet', 'aisle-perf', 'two-player-deck', 'topbar-layout', 'home', 'basic', 'basic-shared-functions', 'tape-motion'] },
];

const codes = await Promise.all(LANES.map((lane) => new Promise((done) => {
  const child = spawn(process.execPath, ['--test', `--test-concurrency=${lane.concurrency}`, ...lane.files.map((f) => `test/${f}.mjs`)], { stdio: ['ignore', 'pipe', 'inherit'] });
  let out = '';
  child.stdout.on('data', (chunk) => { out += chunk; });
  child.on('close', (code) => {
    const lines = out.split('\n'), failing = lines.findIndex((l) => l.startsWith('✖ failing tests:'));
    process.stdout.write(`\n== ${lane.name} lane\n` + lines.filter((l) => /^(✖|ℹ (tests|pass|fail))|^\s+✖/.test(l)).join('\n') + '\n' +
      (failing >= 0 ? lines.slice(failing, failing + 40).join('\n') + '\n' : ''));
    done(code);
  });
})));
process.exit(codes.some((code) => code !== 0) ? 1 : 0);
