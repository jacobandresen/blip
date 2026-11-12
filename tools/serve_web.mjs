// Serve web/ for local play: node tools/serve_web.mjs (PORT=8080 by default).
import { createFileServer, listenOn } from '../test/lib/server.mjs';

const port = Number(process.env.PORT ?? 8080);
await listenOn(createFileServer(), port, '127.0.0.1');
console.log(`BLIP at http://127.0.0.1:${port}`);
