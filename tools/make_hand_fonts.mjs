import { mkdtemp, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { runInherited } from './lib/process.mjs';

if (process.argv.length !== 4) {
  throw new Error('usage: node tools/make_hand_fonts.mjs Caveat[wght].ttf MrsSaintDelafield-Regular.ttf');
}

const [caveat, signature] = process.argv.slice(2);
const root = fileURLToPath(new URL('..', import.meta.url));
const web = join(root, 'web');
const temporary = await mkdtemp(join(tmpdir(), 'blip-fonts-'));

async function subset(source, text, output) {
  try {
    await runInherited('pyftsubset', [source, `--output-file=${output}`, '--flavor=woff2', '--layout-features=*', `--text=${text}`], { cwd: root });
  } catch (error) {
    throw new Error(`could not subset ${source}; install FontTools with WOFF2 support (${error.message})`);
  }
  const { size } = await stat(output);
  console.log(`wrote ${output} (${size.toLocaleString()} bytes)`);
}

try {
  const staticCaveat = join(temporary, 'caveat-medium.ttf');
  await run('fonttools', ['varLib.instancer', caveat, 'wght=500', `--output=${staticCaveat}`]);
  await subset(staticCaveat, Array.from({ length: 95 }, (_, i) => String.fromCharCode(32 + i)).join(''), join(web, 'caveat.woff2'));
  await subset(signature, 'Jacob Andresen', join(web, 'mrs-saint-delafield.woff2'));
} finally {
  await rm(temporary, { recursive: true, force: true });
}
