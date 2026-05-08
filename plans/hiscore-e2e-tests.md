# Hi-Score E2E Test Plan

Tests the full hi-score pipeline (fetch → display → game-over → initials → leaderboard) using
Playwright with all Supabase calls mocked via `page.route()`. No real database writes ever occur.

## Stack

- Playwright (TypeScript)
- `page.route('**/rest/v1/**', ...)` — intercepts every Supabase request before it leaves the browser
- No service-role key, no CI secrets, no cleanup needed

## Fixture: `mockSupabase`

```ts
// test/e2e/fixtures/supabase.ts
interface MockDB {
  hiScores: number[];   // index matches GAME_NAMES order
  top10: { initials: string; score: number }[];
}

export async function mockSupabase(page: Page, overrides: Partial<MockDB> = {}) {
  const db: MockDB = { hiScores: [0, 0, 0, 0], top10: [], ...overrides };

  await page.route('**/rest/v1/**', (route) => {
    const url = route.request().url();

    // GET /hi_scores — returns one row per game
    if (url.includes('/hi_scores')) {
      const GAME_NAMES = ['bouncer', 'serpent', 'galactic_defender', 'canaris'];
      return route.fulfill({
        json: GAME_NAMES.map((game, i) => ({ game, score: db.hiScores[i] })),
      });
    }

    // GET /scores?game=eq.<name> — top-10 check and leaderboard fetch
    if (url.includes('/scores')) {
      return route.fulfill({ json: db.top10 });
    }

    // POST /rpc/set_hi_score
    if (url.includes('/rpc/set_hi_score')) {
      return route.fulfill({ json: {} });
    }

    // POST /rpc/submit_score
    if (url.includes('/rpc/submit_score')) {
      return route.fulfill({ json: {} });
    }

    route.continue();
  });
}
```

## Test cases

### 1. Hi-score badge on index page — no scores

```
mockSupabase(page, { hiScores: [0, 0, 0, 0] })
navigate to /index.html
assert: no .hiscore-badge elements are visible
```

### 2. Hi-score badge on index page — existing score

```
mockSupabase(page, { hiScores: [1234, 0, 0, 0] })
navigate to /index.html
assert: bouncer card shows badge "1,234"
assert: other cards have no badge
```

### 3. Hi-score HUD in game — score loaded from cache

```
mockSupabase(page, { hiScores: [5000, 0, 0, 0] })
navigate to /bouncer/index.html
wait for canvas to be visible
assert: page contains text "5000" (HUD rendered by WASM)
```

### 4. Top-10 overlay appears after game over — score qualifies

```
mockSupabase(page, { top10: [] })  // empty leaderboard → always qualifies
navigate to /bouncer/index.html
inject: window.__blip_test.triggerGameOver(0, 999)
assert: #initials-overlay has class "visible"
assert: #initials-score-val text equals "SCORE 999"
```

### 5. Top-10 overlay does NOT appear — score too low

```
mockSupabase(page, {
  top10: Array(10).fill(null).map((_, i) => ({ initials: 'AAA', score: 9999 - i }))
})
navigate to /bouncer/index.html
inject: window.__blip_test.triggerGameOver(0, 1)
assert: #initials-overlay does NOT have class "visible"
```

### 6. Submit initials → leaderboard shown

```
mockSupabase(page, { top10: [{ initials: 'JAC', score: 999 }] })
navigate to /bouncer/index.html
inject: window.__blip_test.triggerGameOver(0, 999)
wait for #initials-overlay.visible
fill #initials-input with "JAC"
click #initials-submit
assert: #initials-leaderboard is visible
assert: leaderboard contains "JAC" and "999"
assert: matching row has class "lb-new"
```

### 7. CLOSE button dismisses leaderboard

```
(continue from test 6)
click #lb-close
assert: #initials-overlay does NOT have class "visible"
```

## Test hook

Tests 4–7 need to trigger `blip_game_over` from JS. Add a URL-guarded hook to `blip_bridge.js`:

```js
// In blip_bridge.js, after miniquad_add_plugin(...)
if (new URLSearchParams(location.search).has('__blip_test')) {
  window.__blip_test = {
    triggerGameOver: (gameId, score) => checkTop10(gameId, score),
  };
}
```

The `?__blip_test` query param must be present in the test URL; it is never set in production.

## File layout

```
test/
  e2e/
    fixtures/
      supabase.ts      ← mockSupabase helper
    hiscore.spec.ts    ← all 7 tests
playwright.config.ts
```

## Running

```bash
npx playwright test test/e2e/hiscore.spec.ts
```

Start a local static server pointing at `web/` before running (e.g. `python3 -m http.server -d web 8080`),
or configure `webServer` in `playwright.config.ts` to do it automatically.
