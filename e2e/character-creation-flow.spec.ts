import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * Character wizard mount + tab navigation. The most stable entry point is
 * Settings -> "Re-create character" (edit mode), which closes Settings and
 * opens the wizard. Per-tab logic is covered by the wizard unit tests; this
 * verifies the wizard mounts with its full tab strip and tabs switch.
 *
 * `**\/srd/**` is safe to broad-mock - it does not collide with vite's own
 * `/src/...` module URLs.
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });
  await page.route('**/srd/equipment', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ weapons: [], armor: [], adventuring_gear: [] }),
    });
  });
  await page.route('**/srd/**', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
  });
});

test('Settings "Re-create character" opens the wizard with its tab strip', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();

  await page.getByRole('button', { name: /Re-create character/i }).click();

  // Settings closed; the wizard is now visible with its full 10-tab strip.
  const wizard = page.locator('.dm-wizard');
  await expect(wizard).toBeVisible();
  await expect(wizard.getByRole('tab')).toHaveCount(10);
});

test('wizard tabs switch when clicked', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: /Re-create character/i }).click();
  const wizard = page.locator('.dm-wizard');
  await expect(wizard).toBeVisible();

  const last = wizard.getByRole('tab').last();
  await last.click();
  await expect(last).toHaveAttribute('aria-selected', 'true');
});
