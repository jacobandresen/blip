// Deterministic input and image comparison for room captures. Uses the normal pointer/render path.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

export const ROOM_SOURCES = ['index.html', 'arcade-room.css', 'machine.css', 'blip_core.js', 'shell.css', 'shell.js', 'history.html', 'sw.js',
  'aisle.js', 'machine-deck.html', 'deck.js', 'kiosk.css', 'kiosk.js', 'blip_controller.js'];

export function currentRoomHash() {
  const hash = createHash('sha256');
  for (const file of ROOM_SOURCES) hash.update(file).update(readFileSync(new URL(`../../web/${file}`, import.meta.url)));
  return hash.digest('hex');
}

export async function holdHalfway(page, direction = 1) {
  const start = await page.evaluate(() => {
    const grid = document.getElementById('game-grid'), r = grid.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height * .3, unit: grid.clientWidth * .34,
      travel: Number(grid.dataset.roomTravel) };
  });
  await page.mouse.move(start.x, start.y);
  await page.mouse.down();
  await page.mouse.move(start.x - 20 * direction, start.y, { steps: 2 });
  await page.mouse.move(start.x - (20 + start.unit / 2) * direction, start.y, { steps: 6 });
  await page.waitForFunction((target) => Math.abs(Number(document.getElementById('game-grid').dataset.roomTravel) - target) < .01,
    start.travel + direction * .5);
}

export async function attractMask(page, selector = '.card-focused') {
  return page.locator(selector).evaluate((card) => {
    const frame = card.querySelector('.card-loop-frame').getBoundingClientRect();
    return [...card.querySelectorAll('.machine-attract b,.machine-credit')].map((element) => {
      const r = element.getBoundingClientRect();
      return { left: (r.left - frame.left - 8) / frame.width, right: (r.right - frame.left + 8) / frame.width,
        top: (r.top - frame.top - 8) / frame.height, bottom: (r.bottom - frame.top + 8) / frame.height };
    });
  });
}

export async function imageDifference(page, before, after, tolerance = 8, ignore = []) {
  return page.evaluate(async ({ sources, tolerance, ignore }) => {
    let width, height;
    const pixels = await Promise.all(sources.map(async (src) => {
      const image = new Image(); image.src = src; await image.decode();
      const canvas = document.createElement('canvas'); canvas.width = image.width; canvas.height = image.height;
      width = image.width; height = image.height;
      const context = canvas.getContext('2d'); context.drawImage(image, 0, 0);
      return context.getImageData(0, 0, canvas.width, canvas.height).data;
    }));
    if (pixels[0].length !== pixels[1].length) return { fraction: 1, peak: 255 };
    let changed = 0, peak = 0, compared = 0;
    for (let i = 0; i < pixels[0].length; i += 4) {
      if (ignore.length) {
        const x = (i / 4 % width) / width, y = Math.floor(i / 4 / width) / height;
        if (ignore.some((r) => x >= r.left && x <= r.right && y >= r.top && y <= r.bottom)) continue;
      }
      compared++;
      const d = Math.max(...[0, 1, 2].map((c) => Math.abs(pixels[0][i + c] - pixels[1][i + c])));
      peak = Math.max(peak, d);
      if (d > tolerance) changed++;
    }
    return { fraction: compared ? changed / compared : 0, peak };
  }, { sources: [before, after].map((shot) => 'data:image/png;base64,' + shot.toString('base64')), tolerance, ignore });
}
