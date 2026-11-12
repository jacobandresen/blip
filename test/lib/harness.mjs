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
    await cdp.page.evaluate(() => document.getElementById('insert-coin-btn').click());
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

/** Navigate, retrying once when WebKit reports its intermittent "internal error" for a page that loads fine. */
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

export async function gotoPage(page, url, opts) {
  try {
    return await page.goto(url, opts);
  } catch (error) {
    if (!/WebKit encountered an internal error/.test(error.message)) throw error;
    return page.goto(url, opts);
  }
}

/** Wait until the arcade has laid out its machines. */
export function arcadeArranged(page) {
  return page.waitForFunction(() => !document.documentElement.hasAttribute('data-cabinet-arranging'));
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
