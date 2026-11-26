// A static server over web/, shared by the tests, the tools and local play (tools/serve_web.mjs). No dependencies.

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const WEB_DIR = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '..', 'web');

const MIME = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8',
  '.json': 'application/json', '.txt': 'text/plain; charset=utf-8', '.wasm': 'application/wasm',
  '.png': 'image/png', '.webp': 'image/webp', '.jpg': 'image/jpeg', '.svg': 'image/svg+xml', '.ico': 'image/x-icon',
  '.wav': 'audio/wav', '.mp3': 'audio/mpeg', '.woff2': 'font/woff2',
};
const mime = (file) => MIME[path.extname(file).toLowerCase()] || 'application/octet-stream';

/** Start a server and resolve once it is listening. A bare `server.listen(port, r)` never settles when the port
 * is taken, and under `node --test` that is a suite that prints nothing and hangs. */
export function listenOn(server, port, host) {
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
    server.listen(port, host);
  });
}

/** Serve web/ (under `basePath`, as GitHub Pages serves it under /blip). A folder redirects to its slash and
 * serves its index.html. */
export function createFileServer({ basePath = '/' } = {}) {
  return createServer(async (req, res) => {
    try {
      const url = new URL(req.url, 'http://localhost');
      let p = decodeURIComponent(url.pathname);
      if (basePath !== '/') {
        if (p !== basePath && !p.startsWith(`${basePath}/`)) { res.writeHead(404); res.end(); return; }
        p = p.slice(basePath.length) || '/';
      }
      let full = path.resolve(WEB_DIR, `.${p}`);
      if (full !== WEB_DIR && !full.startsWith(`${WEB_DIR}${path.sep}`)) { res.writeHead(403); res.end(); return; }
      if ((await stat(full)).isDirectory()) {
        if (!p.endsWith('/')) { res.writeHead(301, { Location: `${url.pathname}/${url.search}` }); res.end(); return; }
        full = path.join(full, 'index.html');
      }
      const data = await readFile(full);
      res.writeHead(200, { 'Content-Type': mime(full) });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end('not found');
    }
  });
}
