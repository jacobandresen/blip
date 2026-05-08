import type { Page } from '@playwright/test';

export interface MockDB {
  hiScores: number[];
  scores: { game?: string; initials: string; score: number }[];
}

const GAME_NAMES = ['bouncer', 'serpent', 'galactic_defender', 'canaris'];

export async function mockSupabase(page: Page, overrides: Partial<MockDB> = {}): Promise<void> {
  const db: MockDB = {
    hiScores: [0, 0, 0, 0],
    scores: [],
    ...overrides,
  };

  await page.route('**/rest/v1/**', (route) => {
    const url = route.request().url();

    if (url.includes('/hi_scores')) {
      return route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(GAME_NAMES.map((game, i) => ({ game, score: db.hiScores[i] }))),
      });
    }

    if (url.includes('/scores')) {
      return route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(db.scores),
      });
    }

    if (url.includes('/rpc/')) {
      return route.fulfill({
        contentType: 'application/json',
        body: JSON.stringify(null),
      });
    }

    return route.continue();
  });
}
