import { readFile } from 'node:fs/promises';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('real VTT initializes under the desktop content security policy', async ({ page }) => {
  const config = JSON.parse(await readFile('src-tauri/tauri.conf.json', 'utf8'));
  const exceptions: string[] = [];
  page.on('pageerror', (error) => exceptions.push(error.message));
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });
  await page.route(/^http:\/\/127\.0\.0\.1:\d+\/$/, async (route) => {
    const response = await route.fetch();
    await route.fulfill({
      response,
      headers: { ...response.headers(), 'content-security-policy': config.app.security.csp },
    });
  });
  await page.goto('/');
  const canvas = page.locator('canvas').first();
  await expect(canvas).toBeVisible();
  await expect
    .poll(() =>
      canvas.evaluate((element) => (element instanceof HTMLCanvasElement ? element.width : 0)),
    )
    .toBeGreaterThan(0);
  expect(exceptions).toEqual([]);
});
