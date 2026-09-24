// Launches a browser engine through Playwright and hands back a
// CDP-shaped handle, so the helpers in test/lib/harness.mjs
// — which reach the page only through cdp.mjs's `evaluate()`, i.e. only
// through `cdp.send('Runtime.evaluate', { expression })` — can drive
// WebKit and Firefox as well as Chromium, unchanged.
//
// Playwright's WebKit and Firefox are the same engines the phones and
// desktops run, need none of the machine-level setup real Safari
// automation does, and run unattended in CI — so a layout or an input
// path can be checked on all three rather than on Chromium alone.

import { chromium, webkit, firefox } from 'playwright';

const ENGINES = { chromium, webkit, firefox };

/**
 * @param {'chromium'|'webkit'|'firefox'} engineName
 * @param {{ headless?: boolean }} opts
 */
export async function launchEngine(engineName, { headless = true } = {}) {
  const engine = ENGINES[engineName];
  if (!engine) throw new Error(`unknown engine ${engineName}`);

  const args = [];
  if (engineName === 'chromium') {
    // Same flag test/lib/harness.mjs passes its own Chromium:
    // background throttling off, so a headless page that is never
    // "visible" keeps running the game loop.
    args.push('--disable-backgrounding-occluded-windows',
      '--disable-renderer-backgrounding', '--disable-background-timer-throttling');
  }

  const browser = await engine.launch({ headless, args });
  const page = await (await browser.newContext()).newPage();
  return { browser, page, cdp: wrapPage(page, engineName) };
}

/** The CDP-shaped adapter itself — also usable against a Playwright page
 * created some other way. */
export function wrapPage(page, engineName = 'unknown') {
  return {
    kind: engineName,
    page,
    async send(method, params = {}) {
      if (method === 'Runtime.evaluate') {
        try {
          // Wrapped in an IIFE so Playwright always treats the string as
          // an expression to evaluate rather than a function body to
          // call, whatever the caller's expression happens to start with.
          const value = await page.evaluate(`(function () { return (${params.expression}); })()`);
          return { result: { value } };
        } catch (e) {
          return { exceptionDetails: { exception: { description: String((e && e.message) || e) } } };
        }
      }
      if (method === 'Page.navigate') {
        await page.goto(params.url, { waitUntil: 'load', timeout: 60000 });
        return {};
      }
      if (method === 'Runtime.enable' || method === 'Page.enable') return {};
      throw new Error(`playwright engine shim does not implement ${method}`);
    },
  };
}
