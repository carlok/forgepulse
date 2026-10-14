import { expect, test, type Page } from '@playwright/test';

const attention = { version: 'v1', score: 1, rank: null as number | null, components: { unique_views_7d: 1, external_referrer_uniques_14d: 0, new_stars_30d: 0, new_forks_30d: 0 } };

function repository(name: string, cloneRank: number, attentionRank: number) {
  return {
    name, description: `${name} description`, stars: 2, forks: 0,
    total_views: 10, total_view_uniques: 5, total_clones: 100 - cloneRank, total_clone_uniques: 10,
    clones_1d: 1, clones_7d: 2, clones_30d: 3,
    clone_rank: cloneRank, clone_rank_trend: 'stable', clone_share_percent: 1.5, clone_daily_median: 1,
    human_attention: { ...attention, rank: attentionRank }, attention_rank_trend: 'stable', diagnoses: []
  };
}

// both-top-10: alpha, beta. clone-only: gamma. attention-only: delta.
const items = [
  repository('carlok/alpha', 1, 2),
  repository('carlok/beta', 10, 10),
  repository('carlok/gamma', 3, 11),
  repository('carlok/delta', 11, 1)
];

const starEvents = [
  { repository_name: 'carlok/alpha', login: 'newest', avatar_url: '', html_url: 'https://github.com/newest', starred_at: '2026-10-02T10:00:00Z' },
  { repository_name: 'carlok/beta', login: 'oldest', avatar_url: '', html_url: 'https://github.com/oldest', starred_at: '2026-09-01T10:00:00Z' }
];

async function mockApi(page: Page) {
  await page.route('**/api/health', (route) => route.fulfill({ json: { status: 'ok', git_ref: 'test' } }));
  await page.route('**/api/v1/sync-runs', (route) => route.fulfill({ json: [] }));
  await page.route('**/api/v1/star-events', (route) => route.fulfill({ json: starEvents }));
  await page.route('**/api/v1/dashboard**', (route) => route.fulfill({
    json: {
      ranking: 'human_attention', items, total_count: items.length, total_stars: 8, total_forks: 0,
      total_views: 40, total_clones: 400, chart: [], views_chart: [],
      total_clone_statistics: null, unique_clone_statistics: null, repository_clone_daily_median: null
    }
  }));
}

test.beforeEach(async ({ page }) => mockApi(page));

for (const view of ['Human attention', 'Clone volume']) {
  test(`${view}: only repositories in both top 10 get the badge`, async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: view }).click();
    await expect(page.locator('tbody tr')).toHaveCount(items.length);
    const badged = page.locator('tbody tr').filter({ has: page.locator('.top-both') });
    await expect(badged).toHaveCount(2);
    await expect(badged.nth(0)).toContainText('carlok/alpha');
    await expect(badged.nth(1)).toContainText('carlok/beta');
  });
}

test('the Stars total opens a newest-first feed of every star', async ({ page }) => {
  await page.goto('/');
  await page.locator('.kpis button', { hasText: 'Stars' }).click();
  await expect(page).toHaveURL(/\/stars$/);
  const events = page.locator('.star-event');
  await expect(events).toHaveCount(2);
  await expect(events.nth(0)).toContainText('newest');
  await expect(events.nth(1)).toContainText('oldest');
});

test('the sidebar collapses to a top bar on laptop-width windows', async ({ page }) => {
  await page.setViewportSize({ width: 1512, height: 982 });
  await page.goto('/');
  await expect(page.locator('tbody tr').first()).toBeVisible();
  expect((await page.locator('aside').boundingBox())!.width).toBeGreaterThan(1000);

  await page.setViewportSize({ width: 1700, height: 982 });
  expect((await page.locator('aside').boundingBox())!.width).toBeLessThan(400);
});
