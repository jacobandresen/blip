// Which Chromium the Chromium-based tests should launch.
//
// `chromium` on PATH is the right default on the Linux CI boxes this
// suite was written for, but it is almost never on PATH on macOS, where
// the browser is an .app bundle if it is installed at all — so the whole
// multiplayer suite failed to start on a Mac for want of a binary.
// Playwright is already a devDependency (it is what drives the WebKit
// and real-iPhone tests), and it ships a pinned Chromium, so fall back
// to that rather than making every developer install one by hand.
//
// Explicit $BLIP_CHROMIUM still wins, for testing against a specific
// build.

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
