// Child runs for the runner scripts (run-all, run-web, run-visual, shard), from the repository root.

import { spawn } from 'node:child_process';

const ROOT = new URL('../../', import.meta.url);

/** Run a command and resolve its exit code. With `heading`, node --test's output is held back and only its verdict
 * lines and failing-test report are printed under the heading, so parallel runs do not interleave. */
export function run(command, args, { env = process.env, heading = null } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: ROOT, env, stdio: ['ignore', heading ? 'pipe' : 'inherit', 'inherit'] });
    let out = '';
    child.stdout?.on('data', (chunk) => { out += chunk; });
    child.once('error', reject);
    child.once('close', (code) => {
      if (heading) {
        const lines = out.split('\n'), failing = lines.findIndex((l) => l.startsWith('✖ failing tests:'));
        process.stdout.write(`\n== ${heading}\n` + lines.filter((l) => /^(✖|ℹ (tests|pass|fail))|^\s+✖/.test(l)).join('\n') + '\n' +
          (failing >= 0 ? lines.slice(failing, failing + 40).join('\n') + '\n' : ''));
      }
      resolve(code ?? 1);
    });
  });
}
