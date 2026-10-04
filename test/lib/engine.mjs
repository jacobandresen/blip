// Launches a browser engine through Playwright and returns a CDP-shaped
// handle, so harness.mjs helpers (which only call Runtime.evaluate) drive
// WebKit and Firefox as well as Chromium, unattended.

import { chromium, webkit, firefox } from 'playwright';
import { chromiumBinary } from './chromium-binary.mjs';

const ENGINES = { chromium, webkit, firefox };

/**
 * @param {'chromium'|'webkit'|'firefox'} engineName
 * @param {{ headless?: boolean }} opts
 */
export async function launchEngine(engineName, { headless = true, hasTouch = false, viewport } = {}) {
  const engine = ENGINES[engineName];
  if (!engine) throw new Error(`unknown engine ${engineName}`);

  const args = [];
  if (engineName === 'chromium') {
    // Same flag test/lib/harness.mjs passes its own Chromium:
    // background throttling off, so a headless page that is never
    // "visible" keeps running the game loop.
    args.push('--disable-backgrounding-occluded-windows',
      '--disable-renderer-backgrounding', '--disable-background-timer-throttling');
    args.push(hasTouch ? '--touch-events=enabled' : '--touch-events=disabled');
    if (!hasTouch) args.push('--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=1,availablePointerTypes=1');
  }

  const browser = await engine.launch({ headless, args, ...(engineName === 'chromium' ? { executablePath: chromiumBinary() } : {}) });
  const page = await (await browser.newContext({ hasTouch, ...(viewport ? { viewport } : {}) })).newPage();
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
          const value = await page.evaluate((source) => eval(source), params.expression);
          return { result: { value } };
        } catch (e) {
          return { exceptionDetails: { exception: { description: String((e && e.message) || e) } } };
        }
      }
      if (method === 'Page.navigate') {
        await page.goto(params.url, { waitUntil: 'load', timeout: 60000 });
        return {};
      }
      if (method === 'Page.captureScreenshot') {
        const options = {};
        if (params.clip) options.clip = params.clip;
        if (params.format === 'jpeg') options.type = 'jpeg';
        return { data: (await page.screenshot(options)).toString('base64') };
      }
      if (method === 'Input.dispatchKeyEvent') return params.type === 'keyDown' ? page.keyboard.down(params.key) : page.keyboard.up(params.key);
      if (method === 'Input.dispatchMouseEvent') return page.mouse.move(params.x, params.y);
      if (method === 'Input.dispatchTouchEvent') {
        if (params.type === 'touchStart' && params.touchPoints?.[0]) return page.touchscreen.tap(params.touchPoints[0].x, params.touchPoints[0].y);
        return;
      }
      if (method === 'Emulation.setDeviceMetricsOverride') return page.setViewportSize({ width: params.width, height: params.height });
      if (method === 'Emulation.setTouchEmulationEnabled' || method === 'Browser.close') return;
      if (method === 'Runtime.enable' || method === 'Page.enable') return {};
      throw new Error(`playwright engine shim does not implement ${method}`);
    },
  };
}
