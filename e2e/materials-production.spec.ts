import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

for (const width of [960, 1280]) {
  test(`production materials ${width}: portrait, journal, focus and system contrast`, async ({
    page,
  }) => {
    await page.setViewportSize({ width, height: width === 960 ? 640 : 800 });
    await mockTauri(page, {
      onboarding_completed: true,
      hero_class: 'fighter',
      active_provider: 'local-mistralrs',
      image_enabled: false,
      ui_language: 'en',
    });
    await page.goto('/');
    await expect(page.locator('.dm-splash')).toHaveCount(0);
    await page.locator('.dm-char-fab').click();
    await page.getByRole('button', { name: 'Full character sheet', exact: true }).click();
    const sheet = page.locator('.dm-char');
    await expect(sheet).toBeVisible();
    expect(
      await sheet.evaluate((node) => getComputedStyle(node, '::after').borderImageSource),
    ).toContain('frame-panel-v2');
    expect(
      await page
        .locator('.dm-char-portrait')
        .evaluate((node) => getComputedStyle(node, '::after').backgroundImage),
    ).toContain('frame-portrait-v2');
    const close = sheet.locator('button.dm-btn-icon').first();
    await page.keyboard.press('Tab');
    await close.focus();
    await expect(close).toBeFocused();
    await expect(close).toHaveCSS('outline-style', 'solid');
    await page.screenshot({
      path: `.artifacts/product-uiux/materials/sheet-${width}.png`,
      animations: 'disabled',
    });
    await page.emulateMedia({ forcedColors: 'active' });
    expect(await sheet.evaluate((node) => getComputedStyle(node, '::after').display)).toBe('none');
    expect(
      await page
        .locator('.dm-char-portrait')
        .evaluate((node) => getComputedStyle(node, '::after').display),
    ).toBe('none');
    await close.click();
    await expect(sheet).toHaveCount(0);
    await page.emulateMedia({ forcedColors: 'none' });
    await page.getByRole('button', { name: 'Journal', exact: true }).click();
    const journal = page.getByRole('region', { name: "Bard's Journal", exact: true });
    await expect(journal).toBeVisible();
    const container = journal.locator(':scope > div').first();
    expect(
      await container.evaluate((node) => getComputedStyle(node, '::after').borderImageSource),
    ).toContain('frame-journal-v2');
    await page.screenshot({
      path: `.artifacts/product-uiux/materials/journal-${width}.png`,
      animations: 'disabled',
    });
    await page.keyboard.press('Escape');
    await expect(journal).toHaveCount(0);
    await page.getByRole('button', { name: 'Campaigns', exact: true }).click();
    const card = page
      .locator('article')
      .filter({ has: page.getByRole('button', { name: /Continue/i }) })
      .first();
    // Hero card is a real production consumer; verify the painted edge resolves.
    const source = await page
      .locator('main')
      .evaluate(
        (root) =>
          [...root.querySelectorAll('article')]
            .map((node) => getComputedStyle(node, '::after').borderImageSource)
            .find((value) => value.includes('frame-campaign-v2')) ?? '',
      );
    expect(source).toContain('frame-campaign-v2');
    await expect(card).toBeVisible();
    await page.screenshot({
      path: `.artifacts/product-uiux/materials/campaign-${width}.png`,
      animations: 'disabled',
    });
  });
}
