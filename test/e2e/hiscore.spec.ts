import { test, expect } from '@playwright/test';
import { mockSupabase } from './fixtures/supabase';

// Waits for blip_bridge.js to finish executing so __blip_test is available.
async function waitForBridge(page: Parameters<typeof mockSupabase>[0]) {
  await page.waitForFunction(() => typeof (window as any).__blip_test !== 'undefined');
}

// Wait for both Supabase requests the index page makes for badges.
async function waitForIndexBadges(page: Parameters<typeof mockSupabase>[0]) {
  const hi  = page.waitForResponse('**/rest/v1/hi_scores**');
  const sc  = page.waitForResponse('**/rest/v1/scores**');
  await page.goto('/index.html');
  await Promise.all([hi, sc]);
}

// ── Index page badge tests ───────────────────────────────────────────────────

test('no badge shown when all hi-scores are zero', async ({ page }) => {
  await mockSupabase(page);
  await waitForIndexBadges(page);
  await expect(page.locator('.hiscore-badge')).toHaveCount(0);
});

test('badge shows initials and score when leaderboard top matches hi-score', async ({ page }) => {
  await mockSupabase(page, {
    hiScores: [1234, 0, 0, 0],
    scores: [{ game: 'bouncer', initials: 'JAC', score: 1234 }],
  });
  await waitForIndexBadges(page);
  const badge = page.locator('.card-bouncer .hiscore-badge');
  await expect(badge).toBeVisible();
  await expect(badge).toContainText('JAC');
  await expect(badge).toContainText('1,234');
});

test('badge shows score only when no matching leaderboard entry', async ({ page }) => {
  await mockSupabase(page, {
    hiScores: [1234, 0, 0, 0],
    scores: [],
  });
  await waitForIndexBadges(page);
  const badge = page.locator('.card-bouncer .hiscore-badge');
  await expect(badge).toBeVisible();
  await expect(badge).toContainText('1,234');
  await expect(badge).not.toContainText('JAC');
});

test('all four games show badges when they all have hi-scores', async ({ page }) => {
  await mockSupabase(page, {
    hiScores: [100, 200, 300, 400],
    scores: [],
  });
  await waitForIndexBadges(page);
  await expect(page.locator('.hiscore-badge')).toHaveCount(4);
});

// ── Game page load ────────────────────────────────────────────────────────────

test('hi_scores endpoint fetched on game page load', async ({ page }) => {
  await mockSupabase(page, { hiScores: [5000, 0, 0, 0] });
  const resp = page.waitForResponse('**/rest/v1/hi_scores**');
  await page.goto('/bouncer/index.html?__blip_test');
  await resp;
  // confirms blip_bridge.js populated the hi-score cache before WASM calls it
});

// ── Overlay tests ─────────────────────────────────────────────────────────────

test('initials overlay appears when score qualifies for top 10', async ({ page }) => {
  await mockSupabase(page, { scores: [] }); // empty leaderboard → always qualifies
  await page.goto('/bouncer/index.html?__blip_test');
  await waitForBridge(page);

  await page.evaluate(() => (window as any).__blip_test.triggerGameOver(0, 999));

  await expect(page.locator('#initials-overlay')).toHaveClass(/visible/);
  await expect(page.locator('#initials-score-val')).toHaveText('SCORE 999');
});

test('initials overlay does not appear when score is too low', async ({ page }) => {
  const fullBoard = Array.from({ length: 10 }, () => ({ initials: 'AAA', score: 9999 }));
  await mockSupabase(page, { scores: fullBoard });
  await page.goto('/bouncer/index.html?__blip_test');
  await waitForBridge(page);

  const checkDone = page.waitForResponse('**/rest/v1/scores**');
  await page.evaluate(() => (window as any).__blip_test.triggerGameOver(0, 1));
  await checkDone; // wait for async top-10 check to complete

  await expect(page.locator('#initials-overlay')).not.toHaveClass(/visible/);
});

test('submitting initials shows leaderboard with new entry highlighted', async ({ page }) => {
  await mockSupabase(page, {
    scores: [{ initials: 'JAC', score: 999 }],
  });
  await page.goto('/bouncer/index.html?__blip_test');
  await waitForBridge(page);

  await page.evaluate(() => (window as any).__blip_test.triggerGameOver(0, 999));
  await expect(page.locator('#initials-overlay')).toHaveClass(/visible/);

  await page.fill('#initials-input', 'JAC');
  await page.click('#initials-submit');

  await expect(page.locator('#initials-leaderboard')).toBeVisible();
  await expect(page.locator('#lb-list .lb-new')).toBeVisible();
  await expect(page.locator('#lb-list')).toContainText('JAC');
  await expect(page.locator('#lb-list')).toContainText('999');
});

test('close button dismisses the leaderboard overlay', async ({ page }) => {
  await mockSupabase(page, {
    scores: [{ initials: 'JAC', score: 999 }],
  });
  await page.goto('/bouncer/index.html?__blip_test');
  await waitForBridge(page);

  await page.evaluate(() => (window as any).__blip_test.triggerGameOver(0, 999));
  await expect(page.locator('#initials-overlay')).toHaveClass(/visible/);

  await page.fill('#initials-input', 'JAC');
  await page.click('#initials-submit');
  await expect(page.locator('#initials-leaderboard')).toBeVisible();

  await page.click('#lb-close');
  await expect(page.locator('#initials-overlay')).not.toHaveClass(/visible/);
});
