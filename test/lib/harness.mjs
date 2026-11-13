// Shared plumbing for the browser-driven suites: a static server over web/,
// headless browsers, and a CDP-shaped adapter to drive them.

import { evaluate, waitFor, sleep } from './playwright-utils.mjs';
import { launchEngine } from './engine.mjs';
import { createFileServer, listenOn, WEB_DIR } from './server.mjs';

export { evaluate, waitFor, sleep, createFileServer, listenOn, WEB_DIR };
export let HTTP_PORT = 8098; // Updated to the isolated port for each browser suite.

/** Open Rally with a credit inserted, so its controls answer. */
export async function loadRally(cdp) {
  await evaluate(cdp, 'true'); // ensure Runtime is ready before navigating
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${HTTP_PORT}/rally/index.html` });
  await waitFor(cdp, "document.readyState === 'complete'", 15000);
  await waitFor(cdp, "typeof window.BlipController === 'object'", 15000);
  // Game controls stay gated until a real credit is inserted.
  if (await evaluate(cdp, "document.getElementById('need-coin-overlay').classList.contains('visible')")) {
    await insertCoins(cdp.page);
    await waitFor(cdp, "!document.getElementById('need-coin-overlay').classList.contains('visible')");
  }
}

/** Wait for the game's first frame. A minute: a game boots slowly while the other test lane loads the machine.
 * When WebKit's internal load error hit the page (engine.mjs), it is loaded again, once. */
export async function gameReady(page, timeout = 60000) {
  const ready = (t) => page.waitForFunction(() => document.documentElement.hasAttribute('data-game-ready'), null, { timeout: t });
  const started = Date.now();
  while (true) {
    try { return await ready(Math.min(5000, timeout - (Date.now() - started))); } catch (error) {
      if (page.webkitFault) { page.webkitFault = false; await page.reload(); return ready(timeout); }
      if (Date.now() - started >= timeout) throw error;
    }
  }
}

/** Wait for a navigation (from a click, say) to reach `url`. WebKit sometimes fails a navigation with an internal
 * error; then the page is loaded directly, as gotoPage does. */
export async function arrivedAt(page, url, opts) {
  try {
    return await page.waitForURL(url, opts);
  } catch (error) {
    if (typeof url !== 'string' || !/WebKit encountered an internal error/.test(error.message)) throw error;
    return page.goto(url, opts);
  }
}

/** Navigate, retrying once when WebKit reports its intermittent "internal error" for a page that loads fine. */
export async function gotoPage(page, url, opts) {
  try {
    return await page.goto(url, opts);
  } catch (error) {
    if (!/WebKit encountered an internal error/.test(error.message)) throw error;
    return page.goto(url, opts);
  }
}

/** Drop `n` coins through the page's coin button. */
export function insertCoins(page, n = 1) {
  return page.evaluate((n) => { for (let i = 0; i < n; i++) document.getElementById('insert-coin-btn').click(); }, n);
}

/** Wait until the arcade has laid out its machines. */
export function arcadeArranged(page) {
  return page.waitForFunction(() => !document.documentElement.hasAttribute('data-cabinet-arranging'));
}

/** The arcade has laid out its aisle and focused a machine whose Start can be pressed. */
export function machineReady(page) {
  return page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
}

export const PHONE = { width: 390, height: 844 };
export const DESKTOP = { width: 1280, height: 800 };

/** Make a page a phone, as Chrome's device mode does: its metrics, touch, and the notch's safe-area insets. */
export async function emulatePhone(page, { width = PHONE.width, height = PHONE.height, scale = 2, touch = true, insets = null } = {}) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: scale, mobile: touch });
  await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: touch });
  if (insets) await cdp.send('Emulation.setSafeAreaInsetsOverride', { insets });
  return cdp;
}

/** A browser page that is a phone of the given size (see emulatePhone), with its CDP session. */
export async function openPhone(t, { width = PHONE.width, height = PHONE.height } = {}, emulation = {}) {
  const touch = emulation.touch ?? true;
  const handle = await openPage(t, 'chromium', { hasTouch: touch, viewport: { width, height } });
  return { ...handle, cdp: await emulatePhone(handle.page, { width, height, ...emulation }) };
}

/** The arcade with a machine ready to start; touch below 700px wide. */
export async function openArcade(t, { width = DESKTOP.width, height = DESKTOP.height, touch = width < 700, fullscreen = false, query = '', reduce = false, capabilities = null } = {}) {
  const { page, origin } = await openPage(t, 'chromium', { hasTouch: touch, viewport: { width, height } });
  if (reduce) await page.emulateMedia({ reducedMotion: 'reduce' });
  if (fullscreen) await page.addInitScript(() => localStorage.setItem('blip-fullscreen', '1'));
  if (capabilities) await page.addInitScript((values) => {
    for (const [name, value] of Object.entries(values)) Object.defineProperty(navigator, name, { configurable: true, value });
  }, capabilities);
  await page.goto(`${origin}/index.html${query}`);
  await machineReady(page);
  return { page, origin };
}

export async function openPage(t, engine = 'chromium', opts = {}) {
  const handle = await startPage(engine, opts);
  t.after(async () => {
    await handle.close();
  });
  return handle;
}

export async function openBrowser(t, engine = 'chromium', opts = {}) {
  const handle = await launchEngine(engine, opts);
  t.after(() => handle.browser.close().catch(() => {}));
  return handle;
}

export async function startPage(engine = 'chromium', opts = {}) {
  const server = createFileServer();
  await listenOn(server, 0);
  HTTP_PORT = server.address().port;
  let handle;
  try { handle = await launchEngine(engine, opts); }
  catch (error) { await new Promise((r) => server.close(r)); throw error; }
  return {
    ...handle, server, origin: `http://127.0.0.1:${HTTP_PORT}`,
    async close() {
      await handle.browser.close().catch(() => {});
      await new Promise((r) => server.close(r));
    },
  };
}

/** Two browsers with Rally already loaded on both — the shape almost
 * every multiplayer test needs before it can do anything interesting. */
