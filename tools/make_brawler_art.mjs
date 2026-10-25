import { access, copyFile, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { deflateSync, inflateSync } from 'node:zlib';
import { fileURLToPath } from 'node:url';
import { displayCommand, runInherited } from './lib/process.mjs';

const root = fileURLToPath(new URL('..', import.meta.url));
const binary = join(root, 'target', 'release', process.platform === 'win32' ? 'brawler.exe' : 'brawler');

const run = (command, args, options = {}) => runInherited(command, args, { cwd: root, ...options });

const crcTable = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = (c & 1) ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = crcTable[(crc ^ byte) & 255] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const name = Buffer.from(type);
  const body = Buffer.concat([name, data]);
  const head = Buffer.alloc(4);
  head.writeUInt32BE(data.length);
  const tail = Buffer.alloc(4);
  tail.writeUInt32BE(crc32(body));
  return Buffer.concat([head, body, tail]);
}

async function readPng(path) {
  const bytes = await readFile(path);
  let offset = 8;
  let width, height, color, depth;
  const compressed = [];
  while (offset < bytes.length) {
    const size = bytes.readUInt32BE(offset);
    const type = bytes.toString('ascii', offset + 4, offset + 8);
    const data = bytes.subarray(offset + 8, offset + 8 + size);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0); height = data.readUInt32BE(4);
      depth = data[8]; color = data[9];
    }
    if (type === 'IDAT') compressed.push(data);
    offset += size + 12;
    if (type === 'IEND') break;
  }
  if (depth !== 8 || ![2, 6].includes(color)) throw new Error(`unsupported screenshot PNG format in ${path}`);
  const channels = color === 6 ? 4 : 3;
  const bpp = channels;
  const packed = inflateSync(Buffer.concat(compressed));
  const rowBytes = width * channels;
  const decoded = Buffer.alloc(rowBytes * height);
  const paeth = (a, b, c) => {
    const p = a + b - c, da = Math.abs(p - a), db = Math.abs(p - b), dc = Math.abs(p - c);
    return da <= db && da <= dc ? a : db <= dc ? b : c;
  };
  for (let y = 0; y < height; y++) {
    const filter = packed[y * (rowBytes + 1)];
    for (let x = 0; x < rowBytes; x++) {
      const raw = packed[y * (rowBytes + 1) + 1 + x];
      const left = x >= bpp ? decoded[y * rowBytes + x - bpp] : 0;
      const up = y ? decoded[(y - 1) * rowBytes + x] : 0;
      const upperLeft = y && x >= bpp ? decoded[(y - 1) * rowBytes + x - bpp] : 0;
      const predictor = filter === 1 ? left : filter === 2 ? up : filter === 3 ? Math.floor((left + up) / 2)
        : filter === 4 ? paeth(left, up, upperLeft) : 0;
      decoded[y * rowBytes + x] = (raw + predictor) & 255;
    }
  }
  if (channels === 4) return { width, height, pixels: decoded };
  const pixels = Buffer.alloc(width * height * 4);
  for (let i = 0, j = 0; i < decoded.length; i += 3, j += 4) {
    pixels[j] = decoded[i]; pixels[j + 1] = decoded[i + 1]; pixels[j + 2] = decoded[i + 2]; pixels[j + 3] = 255;
  }
  return { width, height, pixels };
}

function writePng(width, height, pixels) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4);
  header[8] = 8; header[9] = 6;
  const rows = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) pixels.copy(rows, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk('IHDR', header), pngChunk('IDAT', deflateSync(rows)), pngChunk('IEND', Buffer.alloc(0))]);
}

function drawCrop(target, targetWidth, source, crop, dest) {
  const [sx, sy, sw, sh] = crop, [dx, dy, dw, dh] = dest;
  for (let y = 0; y < dh; y++) for (let x = 0; x < dw; x++) {
    const src = ((sy + Math.floor(y * sh / dh)) * source.width + sx + Math.floor(x * sw / dw)) * 4;
    const dst = ((dy + y) * targetWidth + dx + x) * 4;
    const alpha = source.pixels[src + 3];
    for (let c = 0; c < 3; c++) target[dst + c] = Math.round(source.pixels[src + c] * alpha / 255 + target[dst + c] * (255 - alpha) / 255);
    target[dst + 3] = 255;
  }
}

async function writeCard(kickPath, titlePath, posterPath, screenshotPath, cardPath) {
  const [kick, title, poster] = await Promise.all([kickPath, titlePath, posterPath].map(readPng));
  const screenshot = Buffer.alloc(640 * 400 * 4);
  for (let i = 3; i < screenshot.length; i += 4) screenshot[i] = 255;
  drawCrop(screenshot, 640, kick, [0, 0, 640, 400], [0, 0, 640, 400]);
  await writeFile(screenshotPath, writePng(640, 400, screenshot));
  const card = Buffer.alloc(640 * 720 * 4);
  for (let i = 3; i < card.length; i += 4) card[i] = 255;
  if (process.env.CARD === 'poster') drawCrop(card, 640, poster, [232, 78, 213, 200], [0, 0, 640, 600]);
  else drawCrop(card, 640, kick, [168, 62, 320, 300], [0, 0, 640, 600]);
  drawCrop(card, 640, title, [0, 8, 640, 114], [0, 604, 640, 114]);
  for (let y = 599; y <= 601; y++) for (let x = 0; x < 640; x++) {
    const at = (y * 640 + x) * 4;
    card[at] = 220; card[at + 1] = 60; card[at + 2] = 40; card[at + 3] = 255;
  }
  await writeFile(cardPath, writePng(640, 720, card));
}

await run('cargo', ['build', '--release', '-p', 'brawler']);
await access(binary);

const env = {
  ...process.env,
  BLIP_SILENT: '1',
  BLIP_EXPORT_BASIC: 'crates/brawler/art',
  BLIP_CRT: '0',
};

async function runNative(gameEnv) {
  const [command, args] = displayCommand(binary, [], { cwd: root });
  await run(command, args, { env: gameEnv });
}

await runNative(env);
await run(process.execPath, ['tools/sync_brawler_art.mjs']);
await run('cargo', ['build', '--release', '-p', 'brawler']);

const captures = await mkdtemp(join(tmpdir(), 'blip-brawler-card-'));
const shot = async (scene, frame, name) => {
  const path = join(captures, `${name}.png`);
  await runNative({ ...process.env, BLIP_SILENT: '1', BLIP_CRT: '0', BLIP_SHOT_SCENE: String(scene),
    BLIP_SCREENSHOT_OUT: path, BLIP_SCREENSHOT_FRAME: String(frame) });
  return path;
};
try {
  const kick = await shot(0, 32, 'kick');
  const title = await shot(3, 150, 'title');
  const poster = await shot(15, 10, 'poster');
  await writeCard(kick, title, poster, join(root, 'web', 'brawler', 'screenshot.png'), join(root, 'web', 'brawler', 'card.png'));
} finally {
  await rm(captures, { recursive: true, force: true });
}
await run(process.execPath, ['tools/make_game_icons.mjs']);
await run('cargo', ['build', '--release', '--target', 'wasm32-unknown-unknown', '-p', 'brawler']);
await copyFile(join(root, 'target', 'wasm32-unknown-unknown', 'release', 'brawler.wasm'),
  join(root, 'web', 'brawler', 'index.wasm'));
await copyFile(join(root, 'web', 'shell.html'), join(root, 'web', 'brawler', 'index.html'));
