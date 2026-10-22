// The arcade's walk costs only style and compositing: no layout while walking or standing still.
// Frame rate is not checked here: headless Chromium draws in software, where 3D cabinets run at half speed.

import test from 'node:test';
import assert from 'node:assert/strict';
import { openPage } from './lib/harness.mjs';

async function metrics(cdp) {
  return Object.fromEntries((await cdp.send('Performance.getMetrics')).metrics.map((m) => [m.name, m.value]));
}

test('walking the aisle on a phone lays nothing out, nor does the arcade at rest', async (t) => {
  const { page, origin } = await openPage(t, 'chromium', { viewport: { width: 390, height: 844 }, hasTouch: true });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Performance.enable');
  await page.goto(`${origin}/index.html`);
  await page.waitForFunction(() => document.querySelector('.card-focused .machine-start') &&
    !document.documentElement.hasAttribute('data-cabinet-arranging'));
  // The arrows' nudge runs five times, then the arcade is still.
  await page.waitForTimeout(9500);

  let before = await metrics(cdp);
  await page.waitForTimeout(2000);
  let after = await metrics(cdp);
  assert.equal(after.LayoutCount - before.LayoutCount, 0, 'layouts while standing still');

  await page.evaluate(() => {
    window.__frames = 0;
    (function count() { if (document.getElementById('game-grid').hasAttribute('data-aisle-moving')) window.__frames++; requestAnimationFrame(count); })();
  });
  before = await metrics(cdp);
  await page.keyboard.down('ArrowRight');
  await page.waitForTimeout(3000);
  await page.keyboard.up('ArrowRight');
  await page.waitForFunction(() => !document.getElementById('game-grid').hasAttribute('data-aisle-moving'), null, { timeout: 8000 });
  after = await metrics(cdp);
  const frames = await page.evaluate(() => window.__frames);
  const layouts = after.LayoutCount - before.LayoutCount;
  const busy = ((after.ScriptDuration - before.ScriptDuration) + (after.RecalcStyleDuration - before.RecalcStyleDuration) +
    (after.LayoutDuration - before.LayoutDuration)) / (after.Timestamp - before.Timestamp);
  assert.ok(frames > 20, `walked for ${frames} frames`);
  assert.ok(layouts <= 3, `${layouts} layouts while walking (one on arrival)`);
  // Alone on a desktop CPU it is about 8%; the suite runs two browsers beside it.
  assert.ok(busy < 0.4, `script and style took ${(busy * 100).toFixed(0)}% of the walk`);
});
