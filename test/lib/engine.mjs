// Launches a browser engine through Playwright and hands back a
// CDP-shaped handle, so the helpers in test/lib/multiplayer-harness.mjs
// — which reach the page only through cdp.mjs's `evaluate()`, i.e. only
// through `cdp.send('Runtime.evaluate', { expression })` — can drive
// WebKit and Firefox as well as Chromium, unchanged.
//
// WebKit is the point of this file. The guest half of two-device
// multiplayer runs on iOS Safari, which is WebKit, and until now the
// whole multiplayer suite ran on Chromium only: WebKit's
// RTCPeerConnection, its SDP output, and its getUserMedia were never
// exercised by any test. Playwright's WebKit is the same engine, runs
// headless, and needs none of the machine-level setup real Safari
// automation does (`sudo safaridriver --enable` plus the Develop menu's
// "Allow Remote Automation"), so it runs unattended in CI.

import { chromium, webkit, firefox } from 'playwright';

const ENGINES = { chromium, webkit, firefox };

/** Chromium's fake-camera flags have no WebKit equivalent; a WebKit run
 * gets a real (empty) media stack and must use scan injection. */
export function supportsFakeCamera(engineName) {
  return engineName === 'chromium';
}

/**
 * @param {'chromium'|'webkit'|'firefox'} engineName
 * @param {{ camFile?: string|null, headless?: boolean, permissions?: string[] }} opts
 */
export async function launchEngine(engineName, { camFile = null, headless = true, permissions = ['camera'] } = {}) {
  const engine = ENGINES[engineName];
  if (!engine) throw new Error(`unknown engine ${engineName}`);

  const args = [];
  if (engineName === 'chromium') {
    // Same flags test/lib/multiplayer-harness.mjs passes its own
    // Chromium — mDNS obfuscation off so host candidates carry real
    // LAN IPs, and background throttling off so a headless page that
    // isn't "visible" keeps running the game loop.
    args.push('--disable-features=WebRtcHideLocalIpsWithMdns',
      '--disable-backgrounding-occluded-windows', '--disable-renderer-backgrounding',
      '--disable-background-timer-throttling');
    if (camFile) {
      args.push('--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream',
        `--use-file-for-fake-video-capture=${camFile}`);
    }
  }

  const browser = await engine.launch({ headless, args });
  // Camera permission is granted by default because it is granted in the
  // real flow too: both pairing roles open the camera to scan a code,
  // and on WebKit the permission is additionally what unblocks ICE
  // gathering entirely (see warmUpIceMedia() in web/blip_net.js). A test
  // that withheld it would be testing a player who tapped "Don't Allow".
  const browser_context = await browser.newContext({ permissions });
  const context = browser_context;
  const page = await context.newPage();
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
