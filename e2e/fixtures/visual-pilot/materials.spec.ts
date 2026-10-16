import { expect, test } from '@playwright/test';

test('review every selected material at 2x2 repeat and frames on light/dark surfaces', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1120, height: 900 });
  await page.goto('/e2e/fixtures/visual-pilot/materials.html');
  await expect(page.locator('[data-material]')).toHaveCount(12);
  await expect(page.locator('[data-frame]')).toHaveCount(4);
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all([...document.images].map((image) => image.decode()));
  });
  for (const section of await page.locator('[data-material],[data-frame]').all()) {
    const id =
      (await section.getAttribute('data-material')) ?? (await section.getAttribute('data-frame'));
    await section.screenshot({ path: `.artifacts/product-uiux/materials/${id}.png` });
  }
  const button = page.getByRole('button', { name: 'Продолжить / Continue' }).first();
  await button.focus();
  await expect(button).toBeFocused();
  await expect(button).toHaveCSS('outline-style', 'solid');
});
