// Desktop WebKit/Chromium at iPhone 14 CSS dimensions, not a real iPhone or iOS Simulator.
import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, readdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { devices } from 'playwright';
import { openPage } from './lib/harness.mjs';
import { attractMask, holdHalfway, imageDifference, currentRoomHash } from './lib/arcade-room.mjs';

const DIR = new URL('artifacts/iphone14-room/', import.meta.url);
const steps = [
  { name: 'portrait', width: 390, height: 844, mode: 'full' },
  { name: 'landscape', width: 844, height: 390, mode: 'full' },
  { name: 'portrait', width: 390, height: 844, mode: 'lite' },
  { name: 'landscape', width: 844, height: 390, mode: 'lite' },
  { name: 'portrait-toolbar', ...devices['iPhone 14'].viewport, mode: 'full' },
  { name: 'landscape-toolbar', ...devices['iPhone 14 landscape'].viewport, mode: 'full' },
  { name: 'portrait-toolbar', ...devices['iPhone 14'].viewport, mode: 'lite' },
  { name: 'landscape-toolbar', ...devices['iPhone 14 landscape'].viewport, mode: 'lite' },
  { name: 'portrait-return', width: 390, height: 844, mode: 'full' },
];
// Select one circuit while diagnosing a layout: BLIP_ROOM_CASE=landscape-toolbar-full.
const cases = process.env.BLIP_ROOM_CASE ? steps.filter(s => `${s.name}-${s.mode}` === process.env.BLIP_ROOM_CASE) : steps;
if (!cases.length) throw new Error(`Unknown BLIP_ROOM_CASE: ${process.env.BLIP_ROOM_CASE}`);

async function paintedRegion(page, region, shot) {
  return page.evaluate(async ({ region, png }) => {
    const image = document.querySelector(`.card[data-machine="${region.machine}"] .card-loop-strip`);
    const frame = image.closest('.card-loop-frame'), rect = frame.getBoundingClientRect(), clip = region.clip;
    const scale = rect.height * 12 / image.naturalHeight;
    const x = rect.left + (rect.width - image.naturalWidth * scale) / 2 - clip.x;
    const y = rect.top + (rect.height - image.naturalHeight * scale) * parseFloat(getComputedStyle(frame).backgroundPositionY) / 100 - clip.y;
    const canvas = document.createElement('canvas'); canvas.width = clip.width; canvas.height = clip.height;
    const context = canvas.getContext('2d');
    context.drawImage(image, x, y, image.naturalWidth * scale, image.naturalHeight * scale);
    const expected = context.getImageData(0, 0, clip.width, clip.height).data;
    const screenshot = new Image(); screenshot.src = png; await screenshot.decode();
    context.clearRect(0, 0, clip.width, clip.height); context.drawImage(screenshot, 0, 0, clip.width, clip.height);
    const actual = context.getImageData(0, 0, clip.width, clip.height).data;
    let lit = 0, painted = 0;
    const samples = [];
    for (let i = 0; i < expected.length; i += 4) {
      const px=i/4%clip.width,py=Math.floor(i/4/clip.width);
      // CSS rounding and DPR downsampling can move a clipped one-pixel paddle across the crop boundary.
      if(px<1||py<1||px>=clip.width-1||py>=clip.height-1)continue;
      if (Math.max(expected[i], expected[i + 1], expected[i + 2]) <= 100) continue;
      lit++;
      if (Math.max(actual[i], actual[i + 1], actual[i + 2]) > 45) painted++;
      if(samples.length<16){
        const x=i/4%clip.width,y=Math.floor(i/4/clip.width),hit=document.elementFromPoint(clip.x+x+.5,clip.y+y+.5);
        samples.push({x,y,expected:[...expected.slice(i,i+3)],actual:[...actual.slice(i,i+3)],hit:hit?.className,machine:hit?.closest('.card')?.dataset.machine});
      }
    }
    return { lit, painted, fraction: lit ? painted / lit : null, source: image.currentSrc, samples };
  }, { region, png: 'data:image/png;base64,' + shot.toString('base64') });
}

for (const engine of ['webkit', 'chromium']) {
  test(`iPhone 14 room screens paint while passing, wrapping and rotating in ${engine}`, async (t) => {
    const { page, origin } = await openPage(t, engine, { viewport: { width: 390, height: 844 }, hasTouch: true,
      deviceScaleFactor: 3, isMobile: true, userAgent: devices['iPhone 14'].userAgent });
    const sourceHash = currentRoomHash();
    const cdp = engine === 'chromium' ? await page.context().newCDPSession(page) : null;
    if (!cdp) await page.route(/\.(html|css)(\?|$)/, async (route) => {
      // Desktop WebKit has no safe-area emulation API: substitute test variables for CSS env() values.
      const response = await route.fetch();
      const body = (await response.text()).replace(/env\(safe-area-inset-(top|right|bottom|left)(?:,\s*[^)]+)?\)/g,
        (_, side) => `var(--test-safe-${side},0px)`);
      await route.fulfill({ response, body });
    });
    const errors = []; page.on('pageerror', (error) => errors.push(error.message));
    await page.goto(`${origin}/index.html?room=full`);
    await page.waitForFunction(() => document.querySelector('.card-focused[data-loop-ready]') &&
      !document.documentElement.hasAttribute('data-cabinet-arranging'));
    await mkdir(DIR, { recursive: true });
    const count = await page.locator('#game-grid .card').count(), report = [];
    for (const config of cases) {
      await page.setViewportSize({ width: config.width, height: config.height });
      const insets = config.width > config.height ? { top: 0, bottom: 21, left: 47, right: 47 }
        : { top: 47, bottom: 34, left: 0, right: 0 };
      if (cdp) {
        await cdp.send('Emulation.setDeviceMetricsOverride', { width: config.width, height: config.height, deviceScaleFactor: 3, mobile: true });
        await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets });
      }
      await page.evaluate(({ mode, insets, substitute }) => {
        document.documentElement.dataset.room = mode;
        window.__safe = insets;
        if (substitute) for (const [side, value] of Object.entries(insets)) document.documentElement.style.setProperty(`--test-safe-${side}`, `${value}px`);
      }, { mode: config.mode, insets, substitute: !cdp });
      await page.waitForFunction((landscape) => document.documentElement.dataset.layout === (landscape ? 'landscape' : 'upright'), config.width > config.height);
      await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
      for (let i = 0; i < count; i++) {
        await holdHalfway(page);
        await page.waitForFunction(() => [...document.querySelectorAll('.card[data-loop]')].every((card) => card.hasAttribute('data-loop-ready')));
        await page.evaluate(() => {
          for (const strip of document.querySelectorAll('.card[data-loop-ready] .card-loop-frame')) {
            const animation = strip.getAnimations()[0]; animation.pause(); animation.currentTime = 1000;
          }
        });
        await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
        const regions = await page.evaluate(() => {
          const grid = document.getElementById('game-grid').getBoundingClientRect();
          return [...document.querySelectorAll('.card[data-loop][data-loop-ready]')].map((card) => {
            const rect = card.querySelector('.card-loop-frame').getBoundingClientRect();
            const x = Math.ceil(Math.max(rect.left + 5, grid.left + 1, window.__safe.left)), y = Math.ceil(Math.max(rect.top + 5, grid.top + 1, window.__safe.top));
            const right = Math.floor(Math.min(rect.right - 5, grid.right - 1, innerWidth - window.__safe.right));
            const bottom = Math.floor(Math.min(rect.bottom - 5, grid.bottom - 1, innerHeight - window.__safe.bottom));
            return { machine: card.dataset.machine, clip: { x, y, width: right - x, height: bottom - y } };
          }).filter((region) => region.clip.width > 18 && region.clip.height > 30);
        });
        assert.ok(regions.length >= 1, `${config.name}: a passing screen is in view`);
        for (const region of regions) {
          const shot = await page.screenshot({ clip: region.clip });
          const paint = await paintedRegion(page, region, shot);
          // A partial crop can legitimately contain only the game's black background.
          if (paint.lit >= 10 && paint.fraction <= .2) {
            await writeFile(new URL('failed-region.png', DIR), shot);
            await page.screenshot({path:fileURLToPath(new URL('failed-room.png', DIR))});
            await writeFile(new URL('failed-region.json', DIR),JSON.stringify({config,region,paint},null,2));
          }
          if (paint.lit >= 10) assert.ok(paint.fraction > .2, `${config.name}, passing ${region.machine}: ${JSON.stringify(paint)}`);
          report.push({ orientation: config.name, mode: config.mode, insets, position: 'half', machine: region.machine, ...paint });
        }
        await page.screenshot({ path: fileURLToPath(new URL(`${engine}-${config.name}-${config.mode}-${i}-half.png`, DIR)) });
        await page.evaluate(() => {
          for (const strip of document.querySelectorAll('.card-loop-frame')) for (const animation of strip.getAnimations()) animation.play();
        });
        await page.mouse.up();
        await page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'));
        await page.waitForSelector('.card-focused[data-loop-ready]');
        const targets = await page.evaluate(() => {
          const card = document.querySelector('.card-focused');
          const grid = document.getElementById('game-grid').getBoundingClientRect();
          return ['.game-rom', '.machine-start', '.machine-coinbox'].map((selector) => {
            const element = card.querySelector(selector), rect = element.getBoundingClientRect();
            const inside = rect.left >= Math.max(grid.left, window.__safe.left) - .5 &&
              rect.right <= Math.min(grid.right, innerWidth - window.__safe.right) + .5 &&
              rect.top >= Math.max(grid.top, window.__safe.top) - .5 &&
              rect.bottom <= Math.min(grid.bottom, innerHeight - window.__safe.bottom) + .5;
            const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
            return { selector, inside, reachesCard: !!hit?.closest('.card-focused'), rect: rect.toJSON() };
          });
        });
        for (const target of targets) {
          // #76's close landscape view ends below Start, with the coin door below the frame.
          if (config.width > config.height && target.selector === '.machine-coinbox' && !target.inside) continue;
          assert.ok(target.inside && target.reachesCard, `${config.name}: safe, unobstructed ${JSON.stringify(target)}`);
        }
        const screen = page.locator('.card-focused .card-loop-frame'), pictures = [];
        const mask = await attractMask(page);
        for (const time of [0, 2000]) {
          await screen.evaluate((image, time) => {
            const animation = image.getAnimations()[0]; animation.pause(); animation.currentTime = time;
          }, time);
          await page.evaluate(() => new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done))));
          pictures.push(await screen.screenshot());
        }
        const difference = await imageDifference(page, pictures[0], pictures[1], 12, mask);
        const machine = await page.locator('.card-focused').getAttribute('data-machine');
        assert.ok(difference.fraction > 0, `${config.name}: ${machine}'s attract picture changes, ${JSON.stringify(difference)}`);
        await screen.evaluate((image) => image.getAnimations()[0].play());
      }
    }
    assert.deepEqual(errors, [], 'no browser errors while walking and rotating');
    assert.equal(currentRoomHash(), sourceHash, 'room sources stayed unchanged throughout the check');
    await writeFile(new URL(`${engine}.json`, DIR), JSON.stringify({ engine, browser: page.context().browser().version(), sourceHash,
      emulation: cdp ? 'desktop Chromium, iPhone 14 mobile viewport, DPR 3 and safe-area overrides'
        : 'desktop WebKit, iPhone 14 mobile viewport, DPR 3, touch and CSS safe-area substitutions', report }, null, 2));
    const images = (await readdir(DIR)).filter((name) => /^(webkit|chromium)-[\w-]+\.png$/.test(name)).sort();
    await writeFile(new URL('index.html', DIR), `<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width">
      <title>iPhone 14 room checks</title><style>body{background:#21150f;color:#fff1df;font:15px system-ui;margin:20px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(280px,1fr));gap:16px}img{width:100%}figure{margin:0}a{color:#ffd37a}</style>
      <h1>iPhone 14 room checks</h1><p>Desktop WebKit/Chromium at 390×844 and 844×390 with touch enabled. These are emulations, not real-device captures.</p>
      <p>${cases.length} circuits per engine. Each passing screen is checked against its decoded game strip; centered screens must paint changing frames.</p>
      <p><a href="../arcade-room/index.html">Room/reference gallery</a> · <a href="webkit.json">WebKit measurements</a> · <a href="chromium.json">Chromium measurements</a></p>
      <main>${images.map((name) => `<figure><a href="${name}"><img src="${name}" loading="lazy" alt="${name}"></a><figcaption>${name}</figcaption></figure>`).join('')}</main>`);
  });
}
