// Each game as its own home-screen app: a small version of its cabinet card
// as the icon, and a manifest that starts the game, not the cabinet.
//
//   node tools/make_game_icons.mjs            (after a screenshot changes)
//
// Writes, per game in web/<slug>/: icon-180.png (apple-touch-icon),
// icon-192.png, icon-512.png, icon-maskable-512.png and manifest.json; and
// the Guide's own (web/guide-icon-*.png, web/guide-manifest.json), so the
// manual can be saved to a home screen as an app of its own.
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

// Full bleed, as a modern home screen expects: the game's picture to the
// edges, and the card's name band along the bottom in the game's colour.
const ICON_HTML = `<!doctype html><html><head><meta charset="utf-8"><style>
@font-face { font-family: 'Press Start 2P'; src: url(/press-start-2p.woff2) format('woff2'); }
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body { width: 512px; height: 512px; overflow: hidden; background: #000; }
.icon { position: absolute; inset: 0; background: #000; }
/* A maskable icon keeps everything that matters inside the middle 80%. */
.card { position: absolute; inset: 0; }
.maskable .window { inset: 0 0 128px 0; }
.maskable .band { height: 128px; padding-bottom: 36px; }
.window { position: absolute; inset: 0 0 116px 0; overflow: hidden; background: #000; }
.window img { position: absolute; }
/* A soft fall into the band, so the picture does not end on a hard line. */
.window::after { content: ''; position: absolute; inset: auto 0 0 0; height: 70px; background: linear-gradient(transparent, rgba(0,0,0,.55)); }
.band { position: absolute; left: 0; right: 0; bottom: 0; height: 116px; display: grid; place-items: center;
  background: linear-gradient(180deg, rgba(var(--accent), .30), rgba(var(--accent), .12) 40%, #0b0a0e), #0b0a0e;
  border-top: 5px solid rgb(var(--accent)); box-shadow: 0 -6px 22px rgba(var(--accent), .45); }
.title { max-width: 440px; text-align: center; white-space: nowrap;
  font: 52px/1 'Press Start 2P', monospace; color: #fff; text-shadow: 0 0 14px rgba(var(--accent), .95), 0 4px 0 rgba(0,0,0,.6); }
</style></head><body><div class="icon"><div class="card">
<div class="window"><img alt=""></div><div class="band"><div class="title"></div></div></div></div></body></html>`;

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
      await document.fonts.load("52px 'Press Start 2P'");
      // Long names shrink to fit; whole pixels keep the pixel font even.
      const room = 440, wide = title.scrollWidth;
      if (wide > room) title.style.fontSize = Math.floor(52 * room / wide) + 'px';
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

  // The Guide: an open book on orange, as in the home screen's dock.
  const GUIDE_HTML = `<!doctype html><html><head><style>
* { margin: 0; } html, body { width: 512px; height: 512px; overflow: hidden; }
.icon { position: absolute; inset: 0; display: grid; place-items: center; background: linear-gradient(160deg, #ffa53a, #ff7a00); }
.maskable svg { width: 260px; }
svg { width: 320px; filter: drop-shadow(0 6px 10px rgba(120, 50, 0, .35)); }
</style></head><body><div class="icon"><svg viewBox="0 0 24 24"><path fill="#fff" d="M12 6.2C10.2 4.8 7.6 4.3 4.5 4.6c-.6 0-1 .5-1 1.1v11.8c0 .6.6 1.1 1.2 1 2.8-.3 5.2.2 6.8 1.5.3.2.7.2 1 0 1.6-1.3 4-1.8 6.8-1.5.6.1 1.2-.4 1.2-1V5.7c0-.6-.4-1.1-1-1.1-3.1-.3-5.7.2-7.5 1.6zm-.8 1.6v10c-1.7-.9-3.8-1.3-6.2-1.2V6.2c2.6-.1 4.7.4 6.2 1.6zm1.6 0c1.5-1.2 3.6-1.7 6.2-1.6v10.4c-2.4-.1-4.5.3-6.2 1.2z"/></svg></div></body></html>`;
  await page.route(`${origin}/__guide.html`, (route) => route.fulfill({ contentType: 'text/html', body: GUIDE_HTML }));
  await page.goto(`${origin}/__guide.html`);
  for (const [file, size, maskable] of [['guide-icon-180.png', 180, false], ['guide-icon-192.png', 192, false], ['guide-icon-512.png', 512, false], ['guide-icon-maskable-512.png', 512, true]]) {
    await page.evaluate((m) => document.querySelector('.icon').classList.toggle('maskable', m), maskable);
    const html = await page.content();
    const shot = await context.browser().newPage({ viewport: { width: 512, height: 512 }, deviceScaleFactor: size / 512 });
    await shot.route(`${origin}/__guide.html`, (route) => route.fulfill({ contentType: 'text/html', body: html }));
    await shot.goto(`${origin}/__guide.html`);
    await writeFile(path.join(WEB_DIR, file), await shot.screenshot());
    await shot.close();
  }
  await writeFile(path.join(WEB_DIR, 'guide-manifest.json'), JSON.stringify({
    id: 'blip-guide', name: 'BLIP Guide', short_name: 'Guide',
    start_url: 'guide.html', scope: 'guide.html', display: 'standalone',
    background_color: '#000000', theme_color: '#000000',
    icons: [
      { src: 'guide-icon-192.png', sizes: '192x192', type: 'image/png' },
      { src: 'guide-icon-512.png', sizes: '512x512', type: 'image/png' },
      { src: 'guide-icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' },
    ],
  }, null, 2) + '\n');
  console.log('[ok] guide');
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
