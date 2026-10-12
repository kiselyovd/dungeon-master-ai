import { mkdir } from 'node:fs/promises';
import { expect, test } from '@playwright/test';

for (const [width, height] of [
  [960, 640],
  [1280, 800],
] as const) {
  for (const language of ['ru', 'en'])
    test(`${language} ${width}x${height}: readable composition and whole miniature`, async ({
      page,
    }) => {
      await page.setViewportSize({ width, height });
      await page.goto(`/e2e/fixtures/visual-pilot/index.html?lang=${language}`);
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await expect(page.locator('.pilot-token img')).toBeVisible();
      await expect(page.locator('.pilot-wall')).toHaveCount(7);
      expect(
        await page
          .locator('.pilot-token img')
          .evaluate(
            (image) =>
              image instanceof HTMLImageElement && image.complete && image.naturalWidth === 512,
          ),
      ).toBe(true);
      const bounds = await page.evaluate(() => ({
        viewport: innerWidth,
        surfaceWidth: document.querySelector('.pilot')?.getBoundingClientRect().width ?? 0,
        width: document.documentElement.scrollWidth,
        body: document.body.scrollWidth,
        controls: [...document.querySelectorAll('.pilot-header button,.pilot-chat-composer')].map(
          (element) => {
            const r = element.getBoundingClientRect();
            return { x: r.left, right: r.right, top: r.top, bottom: r.bottom };
          },
        ),
      }));
      expect(bounds.width).toBeLessThanOrEqual(bounds.viewport);
      expect(bounds.surfaceWidth).toBeGreaterThanOrEqual(bounds.viewport - 1);
      expect(bounds.body).toBeLessThanOrEqual(bounds.viewport);
      for (const control of bounds.controls) {
        expect(control.x).toBeGreaterThanOrEqual(0);
        expect(control.right).toBeLessThanOrEqual(width);
        expect(control.bottom).toBeLessThanOrEqual(height);
      }
      await mkdir('.artifacts/product-uiux/pilot/screens', { recursive: true });
      await page.screenshot({
        path: `.artifacts/product-uiux/pilot/screens/${language}-${width}x${height}.png`,
      });
      await page
        .getByRole('button', { name: language === 'ru' ? 'EN' : 'РУ', exact: true })
        .click();
      await expect(page.getByRole('heading', { level: 1 })).toHaveText(
        language === 'ru' ? 'Riverside Archive' : 'Архив у реки',
      );
    });
}
