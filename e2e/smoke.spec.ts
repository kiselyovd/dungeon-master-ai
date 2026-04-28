import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  // Stub the Tauri internals so the frontend can boot without the
  // real shell. invoke() returns null (so backend_port resolves to
  // null and api/client.ts hangs waiting for the backend-ready event,
  // which is fine for layout-only tests since we never trigger /chat).
  await page.addInitScript(() => {
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      invoke: async () => null,
      transformCallback: () => 0,
    };
  });
});

test('app renders header, grid placeholder, and chat panel', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: /Dungeon Master AI/i })).toBeVisible();
  await expect(page.getByPlaceholder(/Type a message/i)).toBeVisible();
  await expect(page.getByRole('button', { name: /Send/i })).toBeDisabled();
});

test('settings modal opens and closes', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Settings/i }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: /Cancel/i }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
});
