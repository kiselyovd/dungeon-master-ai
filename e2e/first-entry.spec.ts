import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

for (const language of ['ru', 'en'] as const) {
  for (const width of [960, 1280]) {
    test(`first entry ${language} ${width}: contextual adventure without global model or hero setup`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height: width === 960 ? 640 : 800 });
      await mockTauri(page, { ui_language: language, image_enabled: false });
      await page.goto('/');
      await expect(page.locator('.dm-splash')).toHaveCount(0);
      const start = page.getByRole('button', {
        name: language === 'ru' ? 'Начать приключение' : 'Start adventure',
        exact: true,
      });
      const join = page.getByRole('button', {
        name: language === 'ru' ? 'Присоединиться к игре' : 'Join game',
        exact: true,
      });
      await expect(start).toBeVisible();
      await expect(join).toBeVisible();
      for (const button of [start, join]) {
        const box = await button.boundingBox();
        expect(box && box.y + box.height).toBeLessThanOrEqual(width === 960 ? 640 : 800);
      }
      await page.screenshot({
        path: `.artifacts/product-uiux/entry-${language}-${width}.png`,
        animations: 'disabled',
      });
      await start.click();
      await expect(page.locator('.dm-onboarding')).toHaveCount(0);
      await expect(
        page.getByRole('heading', {
          name: language === 'ru' ? 'С чего начнётся история?' : 'Where does the story begin?',
        }),
      ).toBeVisible();
      const prefs = await page.evaluate(() =>
        JSON.parse(localStorage.getItem('dmai-e2e-store:settings.json') ?? '{}'),
      );
      expect(prefs.onboarding_version).toBe(2);
      expect(prefs.onboarding_entry).toBe('adventure');
      expect(prefs.hero_class ?? null).toBeNull();
    });
  }
  test(`guest first entry ${language}: survives restart without local model/solo hero`, async ({
    page,
  }) => {
    await mockTauri(page, { ui_language: language, image_enabled: false });
    await page.goto('/');
    await page
      .getByRole('button', {
        name: language === 'ru' ? 'Присоединиться к игре' : 'Join game',
        exact: true,
      })
      .click();
    await expect(page.locator('.dm-onboarding')).toHaveCount(0);
    await expect(page.locator('input')).not.toHaveCount(0);
    await expect(page.getByRole('tab', { name: /Chat|Модель на компьютере/ })).toHaveCount(0);
    await page.reload();
    await expect(page.locator('.dm-splash')).toHaveCount(0);
    await expect(page.locator('.dm-onboarding')).toHaveCount(0);
    await expect(page.locator('input')).not.toHaveCount(0);
    const prefs = await page.evaluate(() =>
      JSON.parse(localStorage.getItem('dmai-e2e-store:settings.json') ?? '{}'),
    );
    expect(prefs.onboarding_entry).toBe('guest');
    expect(prefs.hero_class ?? null).toBeNull();
  });
}
