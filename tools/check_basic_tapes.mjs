import { spawn } from 'node:child_process';
import { readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { displayCommand } from './lib/process.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const seconds = Number(process.env.BLIP_TAPE_SECS ?? 40);
const seeds = (process.env.BLIP_TAPE_SEEDS ?? '1 2').trim().split(/\s+/).filter(Boolean);
const binary = join(root, 'target', 'release', process.platform === 'win32' ? 'blip_basic.exe' : 'blip_basic');

function run(command, args, { env = process.env, timeout = 0 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: root, env, stdio: ['ignore', 'pipe', 'pipe'], detached: process.platform !== 'win32' });
    let output = '';
    let expired = false;
    child.stdout.setEncoding('utf8').on('data', (chunk) => { output += chunk; });
    child.stderr.setEncoding('utf8').on('data', (chunk) => { output += chunk; });
    const timer = timeout && setTimeout(() => {
      expired = true;
      try {
        if (process.platform === 'win32') child.kill('SIGTERM');
        else process.kill(-child.pid, 'SIGTERM');
      } catch {}
    }, timeout);
    child.once('error', reject);
    child.once('close', (code) => {
      if (timer) clearTimeout(timer);
      resolve({ code: code ?? 1, output, expired });
    });
  });
}

const built = await run('cargo', ['build', '--release', '-p', 'blip_basic']);
if (built.code !== 0) throw new Error('BASIC native build failed');
const tapes = [];
for (const folder of ['web/TAPE', 'test/basic']) {
  const files = await readdir(join(root, folder), { withFileTypes: true });
  tapes.push(...files.filter((entry) => entry.isFile() && entry.name.toUpperCase().endsWith('.BLIP'))
    .map((entry) => ({ file: join(folder, entry.name), check: folder === 'test/basic' })));
}

let failed = false;
for (const tape of tapes) {
  for (const seed of seeds) {
    const env = {
      ...process.env,
      BLIP_SILENT: '1',
      BLIP_BOT: '1',
      BLIP_BOT_FUZZ: seed,
      BLIP_BOT_MAXT: String(seconds),
      BLIP_BASIC_TAPE: tape.file,
    };
    const [command, args] = displayCommand(binary, [], { cwd: root, screen: '720x810x24' });
    const result = await run(command, args, { env, timeout: (seconds * 4 + 30) * 1000 });
    const expected = tape.check ? /outcome=ended/ : /RESULT/;
    const passing = result.code === 0 && !result.expired && expected.test(result.output);
    if (passing) {
      const stats = result.output.split('\n').find((line) => line.includes('RESULT')) ?? 'outcome=ended';
      console.log(`ok   ${tape.file} seed=${seed} ${stats}`);
    } else {
      failed = true;
      const detail = result.output.split('\n').find((line) => /BASIC ERROR|panicked|RESULT|outcome=ended/.test(line)) ?? 'no completion marker';
      console.error(`FAIL ${tape.file} seed=${seed} exit=${result.code}${result.expired ? ' timeout' : ''}: ${detail}`);
    }
    if (tape.check) break;
  }
}
if (failed) process.exitCode = 1;
