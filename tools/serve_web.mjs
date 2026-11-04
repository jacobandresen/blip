import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(fileURLToPath(new URL('../web', import.meta.url)));
const port = Number(process.env.PORT ?? 8080);
const types = new Map([
  ['.css', 'text/css; charset=utf-8'], ['.html', 'text/html; charset=utf-8'],
  ['.js', 'text/javascript; charset=utf-8'], ['.json', 'application/json'],
  ['.png', 'image/png'], ['.webp', 'image/webp'], ['.svg', 'image/svg+xml'],
  ['.wasm', 'application/wasm'], ['.wav', 'audio/wav'], ['.xm', 'application/octet-stream'],
  ['.woff2', 'font/woff2'], ['.txt', 'text/plain; charset=utf-8'],
]);

const server = createServer(async (request, response) => {
  let pathname;
  try { pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname); }
  catch { response.writeHead(400).end('Bad request'); return; }
  const relative = pathname === '/' ? '/index.html' : pathname;
  const file = resolve(root, `.${relative}`);
  if (file !== root && !file.startsWith(`${root}${sep}`)) {
    response.writeHead(403).end('Forbidden');
    return;
  }
  try {
    let path = file;
    let info = await stat(path);
    if (info.isDirectory()) {
      // a game's folder (the home screen launches `warlock/`): its index.html,
      // with the slash a relative page needs
      if (!pathname.endsWith('/')) {
        const query = new URL(request.url, 'http://localhost').search;
        response.writeHead(301, { Location: `${pathname}/${query}` }).end();
        return;
      }
      path = resolve(file, 'index.html');
      info = await stat(path);
    }
    if (!info.isFile()) { response.writeHead(404).end('Not found'); return; }
    response.writeHead(200, { 'Content-Type': types.get(extname(path).toLowerCase()) ?? 'application/octet-stream' });
    createReadStream(path).pipe(response);
  } catch {
    response.writeHead(404).end('Not found');
  }
});

server.listen(port, '127.0.0.1', () => console.log(`BLIP at http://127.0.0.1:${port}`));
