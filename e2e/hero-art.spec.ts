import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

for (const heroClass of ['fighter', 'rogue', 'wizard', 'cleric']) {
  test(`all four ${heroClass} identities are selectable in the actual scoped wizard gallery and survive cold reload`, async ({
    page,
  }) => {
    await mockTauri(page, {
      onboarding_completed: true,
      hero_class: heroClass,
      ui_language: 'en',
      image_enabled: false,
    });
    await page.route('**/srd/equipment', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ weapons: [], armor: [], adventuring_gear: [] }),
      }),
    );
    await page.route('**/srd/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: route.request().url().endsWith('/equipment')
          ? JSON.stringify({ weapons: [], armor: [], adventuring_gear: [] })
          : '[]',
      }),
    );
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByRole('button', { name: 'Re-create character' }).click();
    const wizard = page.locator('.dm-wizard');
    await expect(wizard).toBeVisible();
    await wizard.getByRole('tab', { name: /Portrait/ }).click();
    for (const number of ['01', '02', '03', '04']) {
      const card = wizard.locator(`[data-asset-id="hero.${heroClass}.${number}.v2"]`);
      await expect(card.locator('img')).toBeVisible();
      await expect
        .poll(() =>
          card
            .locator('img')
            .evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth === 512),
        )
        .toBe(true);
      await card.locator('input').check();
      await expect(card.locator('input')).toBeChecked();
      await expect
        .poll(() => page.evaluate(() => localStorage.getItem('dmai-e2e-store:settings.json')))
        .toContain(`hero.${heroClass}.${number}.v2`);
    }
    await page.screenshot({
      path: `.artifacts/visual-production/${heroClass}-gallery.png`,
      animations: 'disabled',
    });
    await page.reload();
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByRole('button', { name: 'Re-create character' }).click();
    await page
      .locator('.dm-wizard')
      .getByRole('tab', { name: /Portrait/ })
      .click();
    await expect(page.locator(`[data-asset-id="hero.${heroClass}.04.v2"] input`)).toBeChecked();
    const cancel = page.locator('.dm-wizard').getByRole('button', { name: 'Cancel', exact: true });
    await expect(cancel).toHaveCount(1, { timeout: 2_000 });
    await cancel.click();
    await expect(page.locator('.dm-wizard')).toHaveCount(0);
    await expect
      .poll(() => page.evaluate(() => localStorage.getItem('dmai-e2e-store:settings.json')))
      .toContain(`hero.${heroClass}.04.v2`);
  });
}
