// Shared plumbing for the browser-driven suites: a static server over web/,
// headless browsers, and a CDP-shaped adapter to drive them.

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { evaluate, waitFor, sleep } from './playwright-utils.mjs';
import { launchEngine } from './engine.mjs';

export { evaluate, waitFor, sleep };

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const WEB_DIR = path.join(__dirname, '..', '..', 'web');
export let HTTP_PORT = 8098; // Updated to the isolated port for each browser suite.

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

/** Start a server and resolve once it is listening. A bare
 * `server.listen(port, r)` never settles when the port is taken, and under
 * `node --test` that is a suite that prints nothing and hangs. */
export function listenOn(server, port) {
  return new Promise((resolve, reject) => {
    const onError = (err) => {
      server.removeListener('listening', onListening);
      reject(new Error(`could not listen on port ${port}: ${err.code || err.message}` +
        (err.code === 'EADDRINUSE' ? ' — another server (or a previous test run) is holding it' : '')));
    };
    const onListening = () => {
      server.removeListener('error', onError);
      resolve(server);
    };
    server.once('error', onError);
    server.once('listening', onListening);
    server.listen(port);
  });
}

export function createFileServer() {
  return createServer(async (req, res) => {
    try {
      let p = decodeURIComponent(req.url.split('?')[0]);
      if (p.endsWith('/')) p += 'index.html';
      const full = path.join(WEB_DIR, p);
      if (!full.startsWith(WEB_DIR)) { res.writeHead(403); res.end(); return; }
      const data = await readFile(full);
      res.writeHead(200, { 'Content-Type': MIME[path.extname(full)] || 'application/octet-stream' });
      res.end(data);
    } catch (e) {
      res.writeHead(404);
      res.end('not found');
    }
  });
}

export async function loadRally(cdp) {
  return loadRallyAt(cdp, `http://127.0.0.1:${HTTP_PORT}`);
}

/** Same as loadRally() but against an explicit origin. */
export async function loadRallyAt(cdp, origin) {
  await evaluate(cdp, 'true'); // ensure Runtime is ready before navigating
  await cdp.send('Page.navigate', { url: `${origin}/rally/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipController === 'object'", 15000);
  // Game controls stay gated until a real credit is inserted.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await cdp.page.locator('#insert-coin-btn').click();
    await waitFor(cdp, "!document.getElementById('need-coin-overlay').classList.contains('visible')");
  }
}

export async function openPage(t, engine = 'chromium', opts = {}) {
  const handle = await startPage(engine, opts);
  t.after(async () => {
    await handle.close();
  });
  return handle;
}

export async function startPage(engine = 'chromium', opts = {}) {
  const server = createFileServer();
  await listenOn(server, 0);
  HTTP_PORT = server.address().port;
  let handle;
  try { handle = await launchEngine(engine, opts); }
  catch (error) { await new Promise((r) => server.close(r)); throw error; }
  return {
    ...handle, server, origin: `http://127.0.0.1:${HTTP_PORT}`,
    async close() {
      await handle.browser.close().catch(() => {});
      await new Promise((r) => server.close(r));
    },
  };
}

/** Two browsers with Rally already loaded on both — the shape almost
 * every multiplayer test needs before it can do anything interesting. */
