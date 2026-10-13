// Runs one test file as several processes, each taking its share of the devices or games (TEST_SHARD).
//
//   node test/shard.mjs 2 test/layout.mjs

import { spawn } from 'node:child_process';

const [count, file] = [Number(process.argv[2]), process.argv[3]];
const codes = await Promise.all(Array.from({ length: count }, (_, shard) => new Promise((done) => {
  const child = spawn(process.execPath, ['--test', '--test-concurrency=1', file], {
    env: { ...process.env, TEST_SHARD: `${shard}/${count}` }, stdio: ['ignore', 'pipe', 'inherit'],
  });
  let out = '';
  child.stdout.on('data', (chunk) => { out += chunk; });
  child.on('close', (code) => {
    process.stdout.write(`\n== shard ${shard + 1}/${count}\n` + out.split('\n').filter((l) => /^(✖|ℹ (tests|pass|fail))|^\s+\+   '/.test(l)).join('\n') + '\n');
    done(code);
  });
})));
process.exit(codes.some((code) => code !== 0) ? 1 : 0);
