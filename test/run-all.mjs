import { spawn } from 'node:child_process';
import { readdir } from 'node:fs/promises';
import { chromium } from 'playwright';
import { createFileServer, listenOn } from './lib/harness.mjs';
import { chromiumBinary } from './lib/chromium-binary.mjs';

const root = new URL('../', import.meta.url);
const args = process.argv.slice(2);
if (args.some((arg) => arg !== '--check-access')) {
  throw new Error('Usage: npm run test:all [-- --check-access]');
}

async function checkAccess() {
  const server = createFileServer();
  await listenOn(server, 0);
  let browser;
  try {
    browser = await chromium.launch({
      executablePath: chromiumBinary(), headless: true, timeout: 30000,
      args: ['--no-sandbox', '--disable-dev-shm-usage'],
    });
    const page = await browser.newPage();
    const response = await page.goto(`http://127.0.0.1:${server.address().port}/basic.html`, {
      waitUntil: 'domcontentloaded', timeout: 30000,
    });
    if (!response?.ok()) throw new Error(`Local page returned ${response?.status()}`);
    console.log('Test access verified: local server, Chromium, and local page loading.');
  } finally {
    await browser?.close();
    await new Promise((resolve) => server.close(resolve));
  }
}

async function run(command, commandArgs) {
  console.log(`\nRunning ${command} ${commandArgs.join(' ')}`);
  return new Promise((resolve, reject) => {
    const child = spawn(command, commandArgs, { cwd: root, stdio: 'inherit' });
    child.once('error', reject);
    child.once('close', (code) => resolve(code ?? 1));
  });
}

await checkAccess();
if (!args.includes('--check-access')) {
  const failures = [];
  if (await run('cargo', ['test', '--release'])) failures.push('Rust');
  if (await run('./build_web.sh', [])) throw new Error('Web build failed');
  const files = (await readdir(new URL('./', import.meta.url)))
    .filter((name) => name.endsWith('.mjs') && !name.startsWith('run-')
      && name !== 'shard.mjs' && name !== 'visual-parity.mjs')
    .sort();
  // Sequential suites keep browser load bounded and print results as they arrive.
  for (const file of files) {
    if (await run(process.execPath, ['--test', '--test-concurrency=1', `test/${file}`])) {
      failures.push(file);
    }
  }
  if (await run(process.execPath, ['test/run-visual.mjs'])) failures.push('visual parity');
  console.log(failures.length ? `\nFailed: ${failures.join(', ')}` : '\nAll test suites passed.');
  process.exitCode = failures.length ? 1 : 0;
}
