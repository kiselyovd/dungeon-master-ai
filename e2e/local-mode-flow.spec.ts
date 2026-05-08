import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      invoke: async () => null,
      transformCallback: () => 0,
    };
  });

  // Mock the Local Mode HTTP surface so the dev backend is not required.
  await page.route('**/local-mode/config', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ selected_llm: 'qwen3_5_4b', vram_strategy: 'auto-swap' }),
    });
  });

  await page.route('**/local/runtime/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        llm: { state: 'ready', port: 37000 },
        image: { state: 'off' },
      }),
    });
  });

  await page.route('**/local/runtime/start', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });

  await page.route('**/local/runtime/stop', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });

  await page.route('**/local/download/qwen3_5_0_8b', async (route) => {
    if (route.request().method() === 'POST') {
      await route.fulfill({ status: 202, body: '' });
      return;
    }
    if (route.request().method() === 'DELETE') {
      await route.fulfill({ status: 204, body: '' });
      return;
    }
    await route.continue();
  });
});

test('local mode modal opens, toggle persists, runtime status shows', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Local Mode/i }).click();
  await expect(page.getByRole('dialog', { name: /local mode/i })).toBeVisible();

  const enableCheckbox = page.getByRole('checkbox', { name: /enable local mode/i });
  await enableCheckbox.check();
  await expect(enableCheckbox).toBeChecked();

  await expect(page.getByText(/LLM/i).first()).toBeVisible();
  await expect(page.locator(':text-is("Image")').first()).toBeVisible();

  // Strategy select offers all three options.
  await expect(page.getByRole('combobox')).toContainText(/Auto-swap/i);
});
