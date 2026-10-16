import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('production wizard selects a stable portrait while media generation is disabled', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
    image_enabled: false,
  });
  await page.route('**/srd/**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: route.request().url().endsWith('/equipment')
        ? '{"weapons":[],"armor":[],"adventuring_gear":[]}'
        : '[]',
    }),
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: /Re-create character/i }).click();
  const wizard = page.locator('.dm-wizard');
  await expect(wizard).toBeVisible();
  await wizard.getByRole('tab', { name: /Portrait$/ }).click();
  const choice = wizard.locator('[data-asset-id="hero.fighter.01.v2"] input');
  await expect(choice).toBeVisible();
  await choice.check();
  await expect(choice).toBeChecked();
  const image = wizard.locator('[data-asset-id="hero.fighter.01.v2"] img');
  expect(
    await image.evaluate(
      (element) =>
        element instanceof HTMLImageElement && element.complete && element.naturalWidth === 512,
    ),
  ).toBe(true);
  await page.screenshot({ path: '.artifacts/product-uiux/portrait-library-production.png' });
});
