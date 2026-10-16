// Each game as its own home-screen app: a small version of its cabinet card
// as the icon, and a manifest that starts the game, not the cabinet.
//
//   node tools/make_game_icons.mjs            (after a screenshot changes)
//
// Writes, per game in web/<slug>/: icon-180.png (apple-touch-icon),
// icon-192.png, icon-512.png, icon-maskable-512.png and manifest.json.
// Generated: edit this file, not its output.

import { writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { chromium } from 'playwright';
import { chromiumBinary } from '../test/lib/chromium-binary.mjs';
import { createFileServer, listenOn, WEB_DIR } from '../test/lib/harness.mjs';

// The part of the picture each icon shows: its centre and width as fractions
// of the image, chosen so the thing that names the game fills the window.
const CROPS = {
  serpent:           { art: 'screenshot.png', cx: 0.55, cy: 0.40, w: 0.62 },
  bouncer:           { art: 'screenshot.png', cx: 0.50, cy: 0.42, w: 0.92 },
  galactic_defender: { art: 'screenshot.png', cx: 0.52, cy: 0.40, w: 0.80 },
  rally:             { art: 'screenshot.png', cx: 0.50, cy: 0.50, w: 0.98 },
  meteors:           { art: 'screenshot.png', cx: 0.70, cy: 0.24, w: 0.52 },
  sky_raider:        { art: 'screenshot.png', cx: 0.46, cy: 0.30, w: 0.56 },
  brawler:           { art: 'card.png',       cx: 0.50, cy: 0.36, w: 0.82 },
  bubbler:           { art: 'screenshot.png', cx: 0.50, cy: 0.70, w: 0.56 },
  adder:             { art: 'screenshot.png', cx: 0.50, cy: 0.40, w: 0.86 },
};

const ICON_HTML = `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face { font-family: 'Press Start 2P'; src: url(/press-start-2p.woff2) format('woff2'); }
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { width: 512px; height: 512px; overflow: hidden; background: #171419; }
/* The card fills the square; a maskable icon keeps it inside the middle 80%. */
.icon { position: absolute; inset: 0; display: grid; place-items: center;
  background: radial-gradient(circle at 50% 38%, rgba(var(--accent), .30), transparent 62%), linear-gradient(155deg, #30251f, #171419 74%); }
.card { position: relative; width: 512px; height: 512px; }
.maskable .card { transform: scale(.8); }
.frame { position: absolute; inset: 26px; border: 5px solid rgba(224, 183, 121, .7); border-radius: 44px;
  box-shadow: inset 0 2px rgba(255, 239, 208, .35), 0 0 0 2px rgba(0, 0, 0, .35); }
.window { position: absolute; left: 62px; right: 62px; top: 62px; height: 290px; overflow: hidden;
  border-radius: 18px; background: #000; box-shadow: 0 0 0 6px #0b0a0c, 0 0 0 9px rgba(var(--accent), .8), 0 0 26px 6px rgba(var(--accent), .45); }
.window img { position: absolute; }
.title { position: absolute; left: 40px; right: 40px; top: 386px; text-align: center; white-space: nowrap;
  font: 48px/1 'Press Start 2P', monospace; color: rgb(var(--accent)); text-shadow: 0 0 10px rgba(var(--accent), .8), 0 4px 0 #000; }
</style></head><body><div class="icon"><div class="card"><div class="frame"></div>
<div class="window"><img alt=""></div><div class="title"></div></div></div></body></html>`;

/** Title case for the label under the icon: SKY RAIDER -> Sky Raider. */
const titleCase = (s) => s.toLowerCase().replace(/\b[a-z]/g, (c) => c.toUpperCase());

const server = createFileServer();
await listenOn(server, 0);
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ executablePath: chromiumBinary(), args: ['--force-color-profile=srgb'] });
try {
  const context = await browser.newContext({ viewport: { width: 512, height: 512 } });
  const page = await context.newPage();
  await page.route(`${origin}/__icon.html`, (route) => route.fulfill({ contentType: 'text/html', body: ICON_HTML }));
  await page.goto(`${origin}/__icon.html`);
  await page.addScriptTag({ url: `${origin}/blip_core.js` });
  const games = await page.evaluate(() => Object.fromEntries(Object.entries(BLIP_GAMES).map(([slug, g]) => [slug, { name: g.name, accent: g.accent }])));

  for (const [slug, game] of Object.entries(games)) {
    const crop = CROPS[slug];
    if (!crop) throw new Error(`no crop for ${slug}: add one to CROPS`);
    if (!existsSync(path.join(WEB_DIR, slug, crop.art))) throw new Error(`${slug}/${crop.art} is missing`);
    await page.evaluate(async ({ slug, game, crop }) => {
      document.documentElement.style.setProperty('--accent', game.accent);
      const title = document.querySelector('.title');
      title.textContent = game.name;
      title.style.fontSize = '';
      await document.fonts.load("48px 'Press Start 2P'");
      // Long names shrink to fit; whole pixels keep the pixel font even.
      const room = title.clientWidth;
      if (title.scrollWidth > room) title.style.fontSize = Math.floor(48 * room / title.scrollWidth) + 'px';
      const img = document.querySelector('.window img'), win = document.querySelector('.window');
      img.src = `/${slug}/${crop.art}`;
      await img.decode();
      const w = win.clientWidth / crop.w, h = w * img.naturalHeight / img.naturalWidth;
      Object.assign(img.style, { width: w + 'px', height: h + 'px',
        left: (win.clientWidth / 2 - crop.cx * w) + 'px', top: (win.clientHeight / 2 - crop.cy * h) + 'px' });
    }, { slug, game, crop });

    const shots = [['icon-180.png', 180, false], ['icon-192.png', 192, false], ['icon-512.png', 512, false], ['icon-maskable-512.png', 512, true]];
    for (const [file, size, maskable] of shots) {
      await page.evaluate((m) => document.querySelector('.icon').classList.toggle('maskable', m), maskable);
      const shotPage = size === 512 ? page : await sized(context, size, page);
      await writeFile(path.join(WEB_DIR, slug, file), await shotPage.screenshot({ omitBackground: false }));
      if (shotPage !== page) await shotPage.close();
    }

    // An id resolves against the origin, not the manifest: name it outright.
    const manifest = {
      id: `blip-${slug}`, name: `${titleCase(game.name)} · BLIP`, short_name: titleCase(game.name),
      start_url: './?app', scope: './', display: 'fullscreen', orientation: 'any',
      background_color: '#050508', theme_color: '#050508',
      icons: [
        { src: 'icon-192.png', sizes: '192x192', type: 'image/png' },
        { src: 'icon-512.png', sizes: '512x512', type: 'image/png' },
        { src: 'icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
      ],
    };
    await writeFile(path.join(WEB_DIR, slug, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
    console.log(`[ok] ${slug}`);
  }
} finally {
  await browser.close();
  await new Promise((r) => server.close(r));
}

/** The same icon drawn at a smaller size: a copy of the page at a fractional
 * device scale, so text and edges are rendered at that size, not shrunk. */
async function sized(context, size, page) {
  const html = await page.content();
  const small = await context.browser().newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: size / 512 });
  await small.route(`${origin}/__icon.html`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
  await small.goto(`${origin}/__icon.html`);
  await small.evaluate(() => Promise.all([document.fonts.ready, ...[...document.images].map((i) => i.decode())]));
  return small;
}
