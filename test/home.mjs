// The home screen: games as apps, the switch to and from the cabinet, free
// play, and each game saved as its own home-screen app.
//
//   node --test test/home.mjs        (after ./build_web.sh and node tools/make_game_icons.mjs)

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { chromium } from 'playwright';
import { openPage, WEB_DIR, listenOn } from './lib/harness.mjs';
import { chromiumBinary } from './lib/chromium-binary.mjs';

const GAMES = ['serpent', 'bouncer', 'galactic_defender', 'rally', 'meteors', 'sky_raider', 'brawler', 'bubbler', 'adder'];

/** A phone with the games' wasm held back: these tests read the pages, not play them. */
async function phone(t, opts = {}) {
  const handle = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 }, ...opts });
  await handle.page.route('**/*.wasm', () => {});
  return handle;
}

/** Pretend to be a game launched from its own home-screen icon. */
const STANDALONE = () => {
  const real = window.matchMedia.bind(window);
  window.matchMedia = (q) => (/display-mode: (standalone|fullscreen)/.test(q)
    ? { matches: true, media: q, addEventListener() {}, removeEventListener() {}, addListener() {}, removeListener() {} } : real(q));
};

const coinWall = (page) => page.evaluate(() => document.getElementById('need-coin-overlay').classList.contains('visible'));

test('the home screen shows every game as an app with its own icon, and nothing from the cabinet', async (t) => {
  const { page, origin } = await phone(t);
  const requested = [];
  page.on('request', (r) => requested.push(new URL(r.url()).pathname));
  await page.goto(`${origin}/home.html`);
  await page.waitForFunction(() => [...document.querySelectorAll('#app-grid img')].every((i) => i.complete && i.naturalWidth > 0));
  const apps = await page.evaluate(() => [...document.querySelectorAll('#app-grid .app-button')].map((b) => ({
    slug: b.dataset.slug, label: b.querySelector('.app-label').textContent, src: b.querySelector('img').getAttribute('src'),
    badge: !!b.parentElement.querySelector('.app-badge'),
  })));
  assert.deepEqual(apps.map((a) => a.slug).sort(), [...GAMES].sort());
  for (const a of apps) assert.match(a.src, new RegExp(`^${a.slug}/icon-192\\.png`));
  assert.deepEqual(apps.filter((a) => a.badge).map((a) => a.slug).sort(), ['adder', 'brawler', 'bubbler', 'rally']);
  assert.ok(await page.locator('#dock-arcade').isVisible(), 'the dock has the way back to the cabinet');
  const cabinet = requested.filter((p) => /kiosk\.(js|css)|wear\.css|deck\.js|manual|howler|blip_controller/.test(p));
  assert.deepEqual(cabinet, [], 'the home screen loads none of the cabinet');
  const sideways = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  assert.equal(sideways, 0);
});

test('Search finds a game by name, and Enter opens it', async (t) => {
  const { page, origin } = await phone(t);
  await page.goto(`${origin}/home.html`);
  await page.locator('#search-pill').click();
  await page.locator('#search-input').fill('bub');
  assert.deepEqual(await page.locator('#search-results button').evaluateAll((bs) => bs.map((b) => b.dataset.slug)), ['bubbler']);
  await page.locator('#search-input').fill('zzz');
  assert.match(await page.locator('#search-results').textContent(), /No games match/);
  await page.locator('#search-cancel').click();
  assert.ok(await page.locator('#search-sheet').isHidden());
  await page.keyboard.press('r');   // typing on the home screen searches
  await page.locator('#search-sheet').waitFor();
  assert.equal(await page.locator('#search-input').inputValue(), 'r');
  await page.locator('#search-input').fill('raid');
  await page.locator('#search-input').press('Enter');
  await page.waitForURL(`${origin}/sky_raider/`);
});

test('the context menu sits beside its icon and inside the screen', async (t) => {
  const { page, origin } = await phone(t);
  await page.goto(`${origin}/home.html`);
  for (const slug of ['serpent', 'rally', 'adder']) {
    await page.locator(`#app-grid [data-slug="${slug}"]`).dispatchEvent('contextmenu');
    await page.locator('#action-sheet').waitFor();
    const [menu, icon] = await Promise.all([page.locator('#action-sheet').boundingBox(), page.locator(`#app-grid [data-slug="${slug}"] .app-icon`).boundingBox()]);
    assert.ok(menu.x >= 0 && menu.x + menu.width <= 390 && menu.y >= 0 && menu.y + menu.height <= 844, `${slug}: the menu is on screen`);
    assert.ok(menu.y >= icon.y + icon.height || menu.y + menu.height <= icon.y, `${slug}: the menu leaves the icon in view`);
    await page.keyboard.press('Escape');
    assert.ok(await page.locator('#action-sheet').isHidden());
  }
});

test('in edit mode an icon is dragged to a new place, which is kept', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
  await page.goto(`${origin}/home.html`);
  await page.locator('#app-grid [data-slug="adder"]').dispatchEvent('contextmenu');
  await page.locator('[data-action="edit"]').click();
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-editing')), true);
  const box = (slug) => page.locator(`#app-grid [data-slug="${slug}"] .app-icon`).boundingBox();
  const from = await box('adder'), to = await box('serpent');
  await page.mouse.move(from.x + from.width / 2, from.y + from.height / 2);
  await page.mouse.down();
  for (let i = 1; i <= 12; i++) await page.mouse.move(from.x + from.width / 2 + (to.x - from.x) * i / 12, from.y + from.height / 2 + (to.y - from.y) * i / 12, { steps: 2 });
  await page.mouse.up();
  await page.locator('#edit-done').click();
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-editing')), false);
  await page.reload();
  const order = await page.locator('#app-grid .app-button').evaluateAll((bs) => bs.map((b) => b.dataset.slug));
  assert.deepEqual(order.slice(0, 2), ['adder', 'serpent']);
  // A tap while editing does not open a game.
  await page.locator('#app-grid [data-slug="rally"]').dispatchEvent('contextmenu');
  await page.locator('[data-action="edit"]').click();
  await page.locator('#app-grid [data-slug="rally"]').click({ force: true });   // it jiggles, so it is never "stable"
  assert.equal(new URL(page.url()).pathname, '/home.html');
});

test('a tap on an icon starts the game on free play, and the logo brings it back home', async (t) => {
  const { page, origin } = await phone(t);
  await page.goto(`${origin}/home.html`);
  await page.locator('[data-slug="serpent"]').first().tap();
  await page.waitForURL(`${origin}/serpent/`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  assert.equal(await coinWall(page), false, 'no coin wall');
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-freeplay')), true);
  assert.equal(await page.evaluate(() => getComputedStyle(document.querySelector('.deck-coin-slot')).visibility), 'hidden', 'no coin slot');
  assert.equal(await page.evaluate(() => blipSpendCoin()), 1, 'a start costs nothing');
  assert.equal(await coinWall(page), false);
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-appstyle')), true, 'an app\'s look, not the cabinet\'s');
  assert.equal(await page.locator('.blip-logo').isVisible(), false);
  assert.equal(await page.locator('#marquee-name').evaluate((el) => getComputedStyle(el, '::after').content), '"Serpent"');
  await page.locator('.app-back').click();
  await page.waitForURL(`${origin}/home.html`);
  assert.equal(await page.evaluate(() => sessionStorage.getItem('blip-cabinet-return')), null, 'no handoff to the cabinet');
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem('blip-recent'))[0]), 'serpent');
  assert.equal(await page.locator('#dock-row [data-slug="serpent"]').count(), 1, 'the game just played is in the dock');
});

test('the cabinet switches to the home screen, which then opens first, and the Arcade icon switches back', async (t) => {
  const { page, origin } = await phone(t);
  await page.goto(`${origin}/index.html`);
  await page.locator('#home-switch-btn').click();
  await page.waitForURL(`${origin}/home.html`);
  await page.goto(`${origin}/index.html`);
  await page.waitForURL(`${origin}/home.html`);
  await page.goto(`${origin}/index.html?cabinet`);
  assert.equal(new URL(page.url()).pathname, '/index.html', '?cabinet opens the cabinet anyway');

  await page.goto(`${origin}/home.html`);
  await page.locator('#dock-arcade').click();
  await page.waitForURL(`${origin}/index.html?cabinet`);
  await page.goto(`${origin}/index.html`);
  assert.equal(new URL(page.url()).pathname, '/index.html', 'the cabinet is the choice again');
  await page.goto(`${origin}/rally/index.html`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  assert.equal(await coinWall(page), true, 'off the home screen the games want coins again');
  assert.equal(await page.evaluate(() => document.querySelector('#marquee-name').getAttribute('href')), '../index.html');
});

test('a long press offers Add to Home Screen, which shows how on the game page', async (t) => {
  const { page, origin } = await phone(t);
  await page.goto(`${origin}/home.html`);
  await page.locator('[data-slug="brawler"]').first().dispatchEvent('contextmenu');
  await page.locator('#action-sheet').waitFor();
  assert.match(await page.locator('#action-title').textContent(), /^Brawler · 1 or 2 players$/);
  await page.locator('#action-sheet [data-action="install"]').click();
  await page.waitForURL(/\/brawler\/\?app$/);
  await page.locator('#install-sheet').waitFor();
  assert.match(await page.locator('#install-title').textContent(), /Add Brawler to your Home Screen/);
  await page.locator('#install-sheet .install-close').click();
  assert.equal(await page.locator('#install-sheet').count(), 0);
});

test('Safari is shown the Share steps; Chrome gets its own Install button', async (t) => {
  const ios = await openPage(t, 'chromium', { hasTouch: true, viewport: { width: 390, height: 844 } });
  await ios.page.route('**/*.wasm', () => {});
  await ios.page.addInitScript(() => Object.defineProperty(navigator, 'userAgent', { get: () => 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1' }));
  await ios.page.goto(`${ios.origin}/adder/?install`);
  await ios.page.locator('.install-steps').waitFor();
  assert.ok(await ios.page.locator('.install-go').isHidden());

  const { page, origin } = await phone(t);
  await page.addInitScript(() => {
    window.__prompted = 0;
    addEventListener('load', () => {
      const e = new Event('beforeinstallprompt', { cancelable: true });
      e.prompt = () => { window.__prompted++; };
      e.userChoice = Promise.resolve({ outcome: 'accepted' });
      dispatchEvent(e);
    });
  });
  await page.goto(`${origin}/adder/?install`);
  await page.locator('.install-go').waitFor({ state: 'visible' });
  await page.locator('.install-go').click();
  await page.waitForFunction(() => window.__prompted === 1 && !document.getElementById('install-sheet'));
});

test('opened from its own icon, a game plays free and stays itself', async (t) => {
  const { page, origin } = await phone(t);
  await page.addInitScript(STANDALONE);
  await page.goto(`${origin}/meteors/?app`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-app')), true);
  assert.equal(await coinWall(page), false);
  assert.equal(await page.evaluate(() => blipFrontendUrl()), './?app');
  assert.equal(await page.locator('.app-back').count(), 0, 'its own app has no Home to go back to');
  await page.locator('#glcanvas').focus();
  // Back to its own title: the page loads again, at the same address.
  await Promise.all([page.waitForEvent('load'), page.keyboard.press('Backspace')]);
  assert.equal(page.url(), `${origin}/meteors/?app`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  assert.equal(await page.evaluate(() => sessionStorage.getItem('blip-cabinet-return')), null);
  assert.equal(await page.locator('#cc-install').count(), 0, 'an installed game does not offer to install itself');

  // The full BLIP app opens the same page from the cabinet, and that is no game app.
  await page.goto(`${origin}/index.html`);
  await page.goto(`${origin}/meteors/`);
  await page.waitForFunction(() => typeof BlipController === 'object');
  assert.equal(await page.evaluate(() => document.documentElement.hasAttribute('data-app')), false);
  assert.equal(await coinWall(page), true);
});

test('a game opened as an app draws a clean, sharp picture: no CRT, full pixel density', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 } });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 3, mobile: false });
  // Record what the game is told when it asks, through the bridge's own import.
  await page.addInitScript(() => {
    let plugin;
    Object.defineProperty(window, 'register_plugin', {
      configurable: true,
      get: () => plugin,
      set: (fn) => { plugin = (imports) => { fn(imports); const ask = imports.env.blip_crt; imports.env.blip_crt = () => (window.__crt = ask()); }; },
    });
  });
  await page.goto(`${origin}/serpent/index.html`);
  await page.waitForFunction(() => window.__crt !== undefined, null, { timeout: 30000 });
  assert.equal(await page.evaluate(() => window.__crt), 1, 'the cabinet keeps its CRT');
  await page.waitForFunction(() => document.getElementById('glcanvas').width > 0);
  assert.equal(await page.evaluate(() => document.getElementById('glcanvas').width === document.getElementById('glcanvas').clientWidth), true, 'the cabinet draws at CSS pixels');

  await page.goto(`${origin}/home.html`);
  await page.locator('[data-slug="serpent"]').first().click();
  await page.waitForURL(`${origin}/serpent/`);
  await page.waitForFunction(() => window.__crt !== undefined, null, { timeout: 30000 });
  assert.equal(await page.evaluate(() => window.__crt), 0);
  await page.waitForFunction(() => { const c = document.getElementById('glcanvas'); return c.width === Math.floor(c.clientWidth * 3); }, null, { timeout: 10000 });
});

test('each game page names itself and links its own manifest and icon', async (t) => {
  const { page, origin } = await phone(t);
  const seen = new Set();
  for (const slug of GAMES) {
    await page.goto(`${origin}/${slug}/`);
    const head = await page.evaluate(() => ({
      title: document.title,
      apple: document.querySelector('meta[name=apple-mobile-web-app-title]').content,
      manifest: document.querySelector('link[rel=manifest]').href,
      icon: document.querySelector('link[rel=apple-touch-icon]').href,
      name: blipGameFromPath(location.pathname).name,
    }));
    assert.equal(head.apple.toUpperCase(), head.name, `${slug}: the saved icon carries the game's name`);
    assert.match(head.title, new RegExp(`^${head.apple} · BLIP$`));
    assert.equal(new URL(head.manifest).pathname, `/${slug}/manifest.json`);
    assert.equal(new URL(head.icon).pathname, `/${slug}/icon-180.png`);

    const manifest = JSON.parse(readFileSync(path.join(WEB_DIR, slug, 'manifest.json'), 'utf8'));
    assert.equal(manifest.id, `blip-${slug}`);
    assert.equal(manifest.scope, './');
    assert.equal(manifest.start_url, './?app');
    assert.equal(manifest.short_name.toUpperCase(), head.name);
    for (const icon of manifest.icons) {
      const [w, h] = pngSize(path.join(WEB_DIR, slug, icon.src));
      assert.equal(`${w}x${h}`, icon.sizes, `${slug}/${icon.src}`);
    }
    const apple = path.join(WEB_DIR, slug, 'icon-180.png');
    assert.deepEqual(pngSize(apple), [180, 180]);
    assert.equal(readFileSync(apple)[25], 2, `${slug}: iOS fills transparency with black, so the touch icon has none`);
    const hash = createHash('sha1').update(readFileSync(path.join(WEB_DIR, slug, 'icon-512.png'))).digest('hex');
    assert.ok(!seen.has(hash), `${slug}: its icon is its own`);
    seen.add(hash);
  }
});

test('Chrome finds each game installable as its own app', async (t) => {
  // The service worker caches /blip/...: serve web/ there, as GitHub Pages does.
  const server = createServer(async (req, res) => {
    let p = decodeURIComponent(req.url.split('?')[0]);
    if (!p.startsWith('/blip/')) { res.writeHead(404); res.end(); return; }
    p = p.slice('/blip'.length);
    if (p.endsWith('/')) p += 'index.html';
    const full = path.join(WEB_DIR, p);
    const type = { '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' }[path.extname(full)] || 'application/octet-stream';
    if (p === '/blip_config.js' && !existsSync(full)) { res.writeHead(200, { 'Content-Type': type }); res.end(''); return; }
    try { const data = await readFile(full); res.writeHead(200, { 'Content-Type': type }); res.end(data); }
    catch { res.writeHead(404); res.end(); }
  });
  await listenOn(server, 0);
  const origin = `http://127.0.0.1:${server.address().port}`;
  const context = await chromium.launchPersistentContext('', { executablePath: chromiumBinary(), headless: true });
  t.after(async () => { await context.close(); await new Promise((r) => server.close(r)); });
  const page = context.pages()[0] || await context.newPage();
  await page.route('**/*.wasm', () => {});
  const cdp = await context.newCDPSession(page);
  for (const slug of ['serpent', 'brawler']) {
    await page.goto(`${origin}/blip/${slug}/`);
    await page.evaluate(() => navigator.serviceWorker.ready);
    const { url, errors } = await cdp.send('Page.getAppManifest');
    assert.equal(new URL(url).pathname, `/blip/${slug}/manifest.json`);
    assert.deepEqual(errors, []);
    const { installabilityErrors } = await cdp.send('Page.getInstallabilityErrors');
    assert.deepEqual(installabilityErrors.map((e) => e.errorId), [], `${slug} installs`);
    const { appId } = await cdp.send('Page.getAppId');
    assert.equal(new URL(appId).pathname, `/blip-${slug}`, 'each game is a separate app');
  }
});

/** Width and height from a PNG's header. */
function pngSize(file) {
  const b = readFileSync(file);
  return [b.readUInt32BE(16), b.readUInt32BE(20)];
}
