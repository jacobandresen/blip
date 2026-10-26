import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { parseArgs } from 'node:util';

const { values } = parseArgs({ options: {
  help: { type: 'boolean' },
  enable: { type: 'boolean' },
  check: { type: 'boolean' },
  port: { type: 'string', default: '4444' },
  url: { type: 'string', default: 'data:text/html,<title>SafariDriver check</title><h1>SafariDriver works</h1>' },
  screenshot: { type: 'string' },
  script: { type: 'string' },
  wait: { type: 'string', default: '1000' },
} });

if (values.help) {
  console.log(`Usage: node tools/safari_driver.mjs [options]
  --enable           Enable Safari remote automation (may prompt for authentication)
  --port <number>    Driver port (default: 4444)
  --check            Open a Safari session, report the page title, then clean up
  --url <url>        Page to visit with --check (default: a local smoke page)
  --screenshot <png> Save a screenshot with --check
  --script <file>    Execute browser JavaScript before capturing; print its return value
  --wait <ms>        Wait after navigation (default: 1000)

Without --enable or --check, serve WebDriver until Ctrl+C.
Requires macOS, Safari, Node.js 20+, and execution outside the agent sandbox.
See tools/safari_driver.md for permission setup.`);
} else {
  try { await main(); }
  catch (error) {
    console.error(`SafariDriver: ${error.message}`);
    console.error('Check Safari remote automation (--enable), macOS prompts, and sandbox approval. See tools/safari_driver.md.');
    process.exitCode = 1;
  }
}

async function main() {
  if (process.platform !== 'darwin') throw new Error('SafariDriver requires macOS.');
  const port = Number(values.port);
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Port must be 1–65535.');
  if (values.enable && (values.check || values.screenshot)) throw new Error('Run --enable separately.');
  if (values.screenshot && !values.check) throw new Error('--screenshot requires --check.');
  if (values.script && !values.check) throw new Error('--script requires --check.');
  const wait = Number(values.wait);
  if (!Number.isFinite(wait) || wait < 0 || wait > 60000) throw new Error('Wait must be 0–60000 milliseconds.');

  const child = spawn('/usr/bin/safaridriver', values.enable ? ['--enable'] : ['--port', String(port)], {
    stdio: 'inherit',
  });
  const completion = once(child, 'exit');
  // Handle spawn errors even while waiting for the HTTP endpoint.
  let spawnError;
  completion.catch(error => { spawnError = error; });
  const stop = () => { if (child.exitCode === null) child.kill('SIGTERM'); };
  const interrupt = () => { process.exitCode = 130; stop(); };
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', interrupt);
  let sessionId;
  const base = `http://127.0.0.1:${port}`;
  async function request(path, method = 'GET', body, timeout = 30000) {
    let response;
    try {
      response = await fetch(`${base}${path}`, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: body === undefined ? undefined : JSON.stringify(body),
        signal: AbortSignal.timeout(timeout),
      });
    } catch (error) {
      throw new Error(`${method} ${path}: ${error.message}`, { cause: error });
    }
    const result = await response.json();
    if (!response.ok || result.value?.error) throw new Error(result.value?.message ?? `HTTP ${response.status}`);
    return result.value;
  }
  try {
    if (values.enable || !values.check) {
      if (!values.enable) console.log(`SafariDriver at ${base}; press Ctrl+C to stop.`);
      const [code, signal] = await completion;
      if (code !== 0 && !process.exitCode) throw new Error(`Driver exited (${signal ?? code}).`);
      return;
    }
    const deadline = Date.now() + 10000;
    while (true) {
      // Never connect to an existing driver after our child has failed to bind.
      await delay(100);
      if (spawnError) throw spawnError;
      if (child.exitCode !== null || child.signalCode !== null) throw new Error('Driver stopped before becoming ready. Check that the port is free.');
      try { await request('/status', 'GET', undefined, 1000); break; }
      catch (error) { if (Date.now() >= deadline) throw error; }
    }
    const session = await request('/session', 'POST', {
      capabilities: { alwaysMatch: { browserName: 'safari', 'safari:automaticInspection': false } },
    }, 90000);
    sessionId = session.sessionId;
    await request(`/session/${sessionId}/timeouts`, 'POST', { pageLoad: 20000 });
    await request(`/session/${sessionId}/url`, 'POST', { url: values.url });
    await delay(wait);
    const title = await request(`/session/${sessionId}/title`);
    console.log(JSON.stringify({ browser: session.capabilities.browserVersion, title }));
    if (values.script) {
      const result = await request(`/session/${sessionId}/execute/sync`, 'POST', {
        script: await readFile(values.script, 'utf8'), args: [],
      });
      console.log(JSON.stringify(result, null, 2));
    }
    if (values.screenshot) {
      const png = await request(`/session/${sessionId}/screenshot`);
      await mkdir(dirname(values.screenshot), { recursive: true });
      await writeFile(values.screenshot, Buffer.from(png, 'base64'));
      console.log(`Screenshot: ${values.screenshot}`);
    }
  } finally {
    if (sessionId) {
      try { await request(`/session/${sessionId}`, 'DELETE', undefined, 3000); }
      catch (error) { console.error(`Session cleanup: ${error.message}`); }
    }
    stop();
    const forceStop = setTimeout(() => child.kill('SIGKILL'), 3000);
    try { await completion; }
    finally {
      clearTimeout(forceStop);
      process.off('SIGINT', interrupt);
      process.off('SIGTERM', interrupt);
    }
  }
}
