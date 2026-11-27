// Launches Chromium through Playwright and returns a CDP-shaped handle, so harness.mjs helpers (which only call
// Runtime.evaluate) drive it unattended.

import { readFileSync } from 'node:fs';
import { chromium } from 'playwright';
import { chromiumBinary } from './chromium-binary.mjs';

const ENGINES = { chromium };

// One browser per set of launch flags, shared by every test of a test file (a process): launching Chromium for each
// test was most of the suites' time. Each test still gets a context of its own, so storage never leaks between
// tests; the browsers close after the file's last test.
const browsers = new Map();

// How fast the pages' CSS and web animations play, for suites that only wait out transitions (playAnimationsAt).
let animationRate = 1;

/** Play every animation in the pages this test file opens `rate` times as fast: for suites whose waits are a shell's
 * transitions, not frames they measure. */
export function playAnimationsAt(rate) { animationRate = rate; }
if (process.env.NODE_TEST_CONTEXT) {
  const { after } = await import('node:test');
  after(() => Promise.all([...browsers.values()].map(async (browser) => (await browser).close().catch(() => {}))));
}

async function launchBrowser(engine, headless, args) {
  const browser = await engine.launch({ headless, args, executablePath: chromiumBinary() });
  // BLIP_DECK names a local build of blip-deck (its dist/blip-deck.js): every context the tests open uses it in place
  // of the tagged one from jsDelivr, to try a new deck before it is released. No service worker, which would fetch
  // the deck itself, past the route.
  if (process.env.BLIP_DECK) {
    const deck = readFileSync(process.env.BLIP_DECK, 'utf8'), newContext = browser.newContext.bind(browser);
    browser.newContext = async (options) => {
      const context = await newContext({ ...options, serviceWorkers: 'block' });
      await context.route(/\/blip-deck@[^/]+\/dist\/blip-deck\.js$/, (route) => route.fulfill({ contentType: 'text/javascript', body: deck }));
      return context;
    };
  }
  return browser;
}

/**
 * A fresh context and page on the shared browser. `close()` closes the context; `browser` is the shared one.
 * @param {'chromium'} engineName
 */
export async function launchEngine(engineName, { headless = true, hasTouch = false, viewport, deviceScaleFactor, userAgent, isMobile } = {}) {
  const engine = ENGINES[engineName];
  if (!engine) throw new Error(`unknown engine ${engineName}`);

  // Background throttling off, so a headless page that is never "visible" keeps running the game loop.
  const args = ['--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding',
    '--disable-background-timer-throttling', '--mute-audio'];
  // BLIP_TEST_GPU=1 draws WebGL on the machine's GPU instead of in software (SwiftShader).
  if (process.env.BLIP_TEST_GPU === '1') args.push('--use-gl=angle', '--use-angle=gl-egl', '--ignore-gpu-blocklist', '--enable-gpu');
  args.push(hasTouch ? '--touch-events=enabled' : '--touch-events=disabled');
  if (!hasTouch) args.push('--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=1,availablePointerTypes=1');

  // Outside node --test (a tool such as css_check), a browser of its own, closed with the page.
  const shared = Boolean(process.env.NODE_TEST_CONTEXT);
  const key = JSON.stringify([engineName, headless, args]);
  if (shared && !browsers.has(key)) browsers.set(key, launchBrowser(engine, headless, args));
  const browser = shared ? await browsers.get(key) : await launchBrowser(engine, headless, args);
  const context = await browser.newContext({ hasTouch, ...(viewport ? { viewport } : {}),
    ...(deviceScaleFactor === undefined ? {} : { deviceScaleFactor }), ...(userAgent ? { userAgent } : {}),
    ...(isMobile === undefined ? {} : { isMobile }) });
  const page = await context.newPage();
  if (animationRate !== 1) {
    const session = await context.newCDPSession(page);
    await session.send('Animation.enable');
    await session.send('Animation.setPlaybackRate', { playbackRate: animationRate });
  }
  return { browser, context, page, cdp: wrapPage(page, engineName), close: () => (shared ? context : browser).close().catch(() => {}) };
}

/** A CDP-shaped adapter over a Playwright page, for the suites that drive pages through send(). */
function wrapPage(page, engineName = 'unknown') {
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
      if (method === 'Input.insertText') return page.keyboard.insertText(params.text);
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
