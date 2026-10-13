import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const ids = ['mila', 'agata', 'leon', 'varden', 'tikhon', 'savely'] as const;
for (const lang of ['ru', 'en'] as const) {
  test(`Bell public portraits ${lang}: hidden identities, all six consumers, cold restore`, async ({
    page,
  }) => {
    const adventure = {
      state: {
        package_id: 'bell-beneath-water',
        location: 'town',
        facts: [],
        ending: null,
        closed: false,
        history: [],
      },
      available_actions: [],
      public_goals: [],
      public_npcs: ids.slice(0, 2).map((id) => ({ id, portrait_asset_id: `npc.bell.${id}.v2` })),
    };
    await mockTauri(page, {
      onboarding_completed: true,
      hero_class: 'fighter',
      ui_language: lang,
      public_adventure: adventure,
    });
    await page.route('**/sessions/**', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ messages: [] }),
      }),
    );
    await page.goto('/');
    await page
      .getByRole('button', { name: lang === 'ru' ? 'Приключение' : 'Adventure', exact: true })
      .and(page.locator('[aria-expanded]'))
      .click();
    await page.getByRole('tab', { name: lang === 'ru' ? 'Сцена' : 'Scene', exact: true }).click();
    await expect(page.locator('img[data-asset-id="npc.bell.mila.v2"]')).toHaveCount(1);
    for (const id of ids.slice(2))
      await expect(page.locator(`img[data-asset-id="npc.bell.${id}.v2"]`)).toHaveCount(0);
    adventure.public_npcs = ids.map((id) => ({ id, portrait_asset_id: `npc.bell.${id}.v2` }));
    await page.reload();
    await page
      .getByRole('button', { name: lang === 'ru' ? 'Приключение' : 'Adventure', exact: true })
      .and(page.locator('[aria-expanded]'))
      .click();
    await page.getByRole('tab', { name: lang === 'ru' ? 'Сцена' : 'Scene', exact: true }).click();
    for (const id of ids) {
      const image = page.locator(`img[data-asset-id="npc.bell.${id}.v2"]`).first();
      await expect(image).toHaveCount(1);
      await expect
        .poll(() =>
          image.evaluate((img: HTMLImageElement) => img.complete && img.naturalWidth === 768),
        )
        .toBe(true);
      await image.locator('..').click();
      const dialog = page.getByRole('dialog');
      await expect(dialog.locator('img')).toHaveAttribute('data-asset-id', `npc.bell.${id}.v2`);
      await page.screenshot({
        path: `.artifacts/visual-production/bell-npc-${id}-${lang}.png`,
        animations: 'disabled',
      });
      await page.keyboard.press('Escape');
      await expect(dialog).toHaveCount(0);
    }
    await page.reload();
    await page
      .getByRole('button', { name: lang === 'ru' ? 'Приключение' : 'Adventure', exact: true })
      .and(page.locator('[aria-expanded]'))
      .click();
    await page.getByRole('tab', { name: lang === 'ru' ? 'Сцена' : 'Scene', exact: true }).click();
    await expect(page.locator('img[data-asset-id="npc.bell.savely.v2"]')).toHaveCount(1);
  });
}
