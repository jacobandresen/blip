// Runs one test file as several processes, each taking its share of the devices or games (TEST_SHARD).
//
//   node test/shard.mjs 2 test/layout.mjs

import { run } from './lib/run.mjs';

const [count, file] = [Number(process.argv[2]), process.argv[3]];
const codes = await Promise.all(Array.from({ length: count }, (_, shard) => run(process.execPath,
  ['--test', '--test-concurrency=1', file], { env: { ...process.env, TEST_SHARD: `${shard}/${count}` }, heading: `shard ${shard + 1}/${count}` })));
// exitCode, not exit(): a piped stdout must drain the last shard's report first
process.exitCode = codes.some((code) => code !== 0) ? 1 : 0;
