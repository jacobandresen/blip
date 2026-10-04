// Which Chromium the Chromium-based tests launch: $BLIP_CHROMIUM if set, else
// `chromium` on PATH (Linux CI), else Playwright's pinned Chromium (macOS
// rarely has one on PATH).

import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import path from 'node:path';

function onPath(name) {
  for (const dir of (process.env.PATH || '').split(path.delimiter)) {
    const candidate = path.join(dir, name);
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

export function chromiumBinary() {
  if (process.env.BLIP_CHROMIUM) return process.env.BLIP_CHROMIUM;
  const system = onPath('chromium');
  if (system) return system;
  try {
    const require = createRequire(import.meta.url);
    const p = require('playwright').chromium.executablePath();
    if (p && existsSync(p)) return p;
  } catch { /* playwright not installed, or its browsers not downloaded */ }
  return 'chromium'; // let the launch fail with the familiar ENOENT
}
