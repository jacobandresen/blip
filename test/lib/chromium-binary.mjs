// Which Chromium the Chromium-based tests launch: $BLIP_CHROMIUM if set, else
// `chromium` on PATH (Linux CI), else Playwright's pinned Chromium (macOS
// rarely has one on PATH).

import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

function onPath(name) {
  try {
    execFileSync('command', ['-v', name], { stdio: 'ignore', shell: '/bin/sh' });
    return true;
  } catch { return false; }
}

export function chromiumBinary() {
  if (process.env.BLIP_CHROMIUM) return process.env.BLIP_CHROMIUM;
  if (onPath('chromium')) return 'chromium';
  try {
    const require = createRequire(import.meta.url);
    const p = require('playwright').chromium.executablePath();
    if (p && existsSync(p)) return p;
  } catch { /* playwright not installed, or its browsers not downloaded */ }
  return 'chromium'; // let the launch fail with the familiar ENOENT
}
