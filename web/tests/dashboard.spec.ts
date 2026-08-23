import { expect, test } from '@playwright/test';

test('dashboard exposes its core controls', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Export JSONL' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Total clones' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Unique cloners' })).toBeVisible();
});

