// Thin Node wrapper around scripts/lib/android-emulator.sh's CLI, so a
// node:test file can drive AVD lifecycle (boot/wait/permissions/etc.)
// without reimplementing any of that logic — the bash library stays the
// single source of truth, shared with the manual
// scripts/setup-android-multiplayer-test.sh flow.

import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const LIB = path.join(__dirname, '..', '..', 'scripts', 'lib', 'android-emulator.sh');

function run(args, { background = false } = {}) {
  return new Promise((resolve, reject) => {
    let out = '';
    const p = spawn('bash', [LIB, ...args], { stdio: ['ignore', 'pipe', 'inherit'] });
    p.stdout.on('data', (d) => { out += d; process.stderr.write(`[android-emulator ${args[0]}] ${d}`); });
    p.on('error', reject);
    p.on('exit', (code) => {
      if (code === 0) resolve(out.trim());
      else reject(new Error(`android-emulator.sh ${args.join(' ')} exited ${code}`));
    });
    if (background) resolve(p); // caller doesn't want to wait for exit
  });
}

export const ensureSdk = () => run(['ensure-sdk']);
export const createAvdIfMissing = (name) => run(['create-avd', name]);
// Boots the AVD in its own backgrounded process (the shell script itself
// starts the emulator with a trailing `&` and immediately returns its
// PID) — this call resolves as soon as that PID line is printed, well
// before the emulator finishes booting. Follow with waitForBoot().
export const boot = (name, port, logfile, camera) => run(['boot', name, String(port), logfile || '', camera || 'none']);
export const waitForBoot = (serial, maxTries) => run(['wait-for-boot', serial, ...(maxTries ? [String(maxTries)] : [])]);
export const skipSetupWizard = (serial) => run(['skip-setup-wizard', serial]);
export const openUrlDismissOnboarding = (serial, url) => run(['open', serial, url]);
export const resetChrome = (serial) => run(['reset-chrome', serial]);
export const reversePort = (serial, port) => run(['reverse-port', serial, String(port)]);
export const forwardDevtools = (serial, localPort) => run(['forward-devtools', serial, String(localPort)]);
export const grantCameraPermission = (serial) => run(['grant-camera-permission', serial]);
export const kill = (serial) => run(['kill', serial]);

export const sdkRoot = () => process.env.ANDROID_SDK_ROOT || path.join(process.env.HOME || '', 'Android', 'Sdk');
export const adbPath = () => path.join(sdkRoot(), 'platform-tools', 'adb');
