// Shared plumbing for the browser-driven test suites: a static file
// server over web/, one or more headless browsers, and a CDP-shaped
// adapter to drive them. Every suite opens with the same few lines —
// make a server, listen, launch, tear both down after — and written
// once, the class of bug that used to live there (a listen() with no
// error path, which turned a held port into a suite that produced no
// output and never finished) has one place to live.

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { evaluate, waitFor, sleep, killAll } from './cdp.mjs';
import { launchEngine } from './engine.mjs';

export { evaluate, waitFor, sleep, killAll };

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const WEB_DIR = path.join(__dirname, '..', '..', 'web');
export const HTTP_PORT = 8098; // distinct from the 8080 devs run by hand

const MIME = {
  '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.ico': 'image/x-icon', '.woff2': 'font/woff2',
};

/** Start a server and resolve once it is actually listening.
 *
 * The obvious `new Promise((r) => server.listen(port, r))` has no error
 * path: if the port is taken, 'error' fires, the callback never runs,
 * and the promise never settles. Under `node --test` that surfaces as a
 * suite producing *no output at all* and hanging until something kills
 * it — the worst possible failure mode, and one that looks like a
 * device or network problem rather than a stray process holding a port.
 * (It was a stray process holding a port.) */
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
  // The coin wall: shell.js swallows every keydown while it's up — dismiss
  // it the same way a real player would, a click/tap inserting a coin.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await evaluate(cdp, "document.getElementById('need-coin-overlay').click()");
  }
}

export async function openPage(t, engine = 'chromium', opts = {}) {
  const server = createFileServer();
  await listenOn(server, HTTP_PORT);
  const handle = await launchEngine(engine, opts);
  t.after(async () => {
    await handle.browser.close().catch(() => {});
    await new Promise((r) => server.close(r));
  });
  return { ...handle, server };
}

/** Two browsers with Rally already loaded on both — the shape almost
 * every multiplayer test needs before it can do anything interesting. */
