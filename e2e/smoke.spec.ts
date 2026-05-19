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
  // Title is a span (DUNGEON MASTER AI brand mark), not a semantic heading.
  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  // Composer placeholder text changed from "Type a message" to "What do you do?".
  await expect(page.getByPlaceholder(/what do you do/i)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Send' })).toBeDisabled();
});

test('settings modal opens and closes', async ({ page }) => {
  await page.goto('/');
  // The header has one "Settings" button; the status bar adds three more
  // (Chat provider settings / Image gen settings / Video gen settings) that
  // also match a loose /Settings/i regex. Use exact name to target the
  // titlebar button only.
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.getByRole('button', { name: /Cancel/i }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible();
});
