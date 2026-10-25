import { spawn, spawnSync } from 'node:child_process';

export function runInherited(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: 'inherit', ...options });
    child.once('error', reject);
    child.once('exit', (code, signal) => code === 0
      ? resolve()
      : reject(new Error(`${command} exited ${signal ? `on ${signal}` : `with code ${code}`}`)));
  });
}

export function displayCommand(command, args, { cwd, screen = '960x600x24' } = {}) {
  if (process.platform !== 'linux') return [command, args];
  const xvfb = spawnSync('xvfb-run', ['--help'], { cwd, stdio: 'ignore' });
  if (!xvfb.error) return ['xvfb-run', ['-a', '-s', `-screen 0 ${screen}`, command, ...args]];
  if (process.env.DISPLAY || process.env.WAYLAND_DISPLAY) return [command, args];
  throw new Error('A display is required on Linux; install Xvfb or run in a desktop session.');
}
