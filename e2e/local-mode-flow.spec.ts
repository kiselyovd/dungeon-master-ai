import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.beforeEach(async ({ page }) => {
  // active_provider keeps PreflightCheck satisfied (local-mistralrs is
  // exempt from the missing_chat check) so its blocking modal never mounts.
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
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
      body: JSON.stringify({ selected_llm: 'gemma4_e2b', vram_strategy: 'auto-swap' }),
    });
  });

  await page.route('**/local/runtime/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        model_id: 'gemma4_e2b',
        llm: { state: 'ready', port: 37000 },
        image: { state: 'off' },
      }),
    });
  });

  await page.route('**/local-llm/manifest', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        system: [
          {
            id: 'gemma-4-e2b',
            display_name: 'Gemma 4 E2B',
            hf_repo: 'fixture/model',
            hf_filename: 'fixture.gguf',
            arch: 'gemma',
            quant: 'q4',
            size_gb: 3,
            license: 'apache-2.0',
          },
        ],
        user: [],
        installed_ids: ['gemma-4-e2b'],
        download_states: {},
      }),
    }),
  );

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

test('local mode shows runtime status and saves configuration that survives reopening', async ({
  page,
}) => {
  let configured: unknown;
  let starts = 0;
  await page.route('**/local-mode/config', async (route) => {
    if (route.request().method() === 'POST') configured = route.request().postDataJSON();
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
  await page.route('**/local/runtime/start', async (route) => {
    starts++;
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Commands', exact: true })).toBeVisible();
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  // Local Mode modal has no titlebar button - the only documented entry is
  // the Ctrl+Shift+M keyboard shortcut wired in App.tsx.
  await page.keyboard.press('Control+Shift+M');
  const dialog = page.getByRole('dialog', { name: 'Local Mode', exact: true });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Check status', exact: true }).click();
  await expect(dialog.getByText('Chat', { exact: true }).locator('..')).toHaveText('Chat Running');
  await expect(dialog.getByRole('radio', { name: /Gemma 4 E2B/ })).toBeChecked();
  await dialog.getByRole('button', { name: 'Advanced', exact: true }).click();
  await expect(dialog.getByRole('heading', { name: 'Image model', exact: true })).toBeVisible();
  const strategy = dialog.getByRole('combobox', { name: 'Graphics memory', exact: true });
  await expect(strategy).toHaveText('Free memory for images');
  await strategy.click();
  await expect(dialog.getByRole('option')).toHaveCount(3);
  await dialog.getByRole('option', { name: 'Keep models loaded', exact: true }).click();
  await dialog.getByRole('button', { name: 'Start model', exact: true }).click();
  await expect.poll(() => starts).toBe(1);
  expect(configured).toEqual({
    selected_llm: 'gemma4_e2b',
    vram_strategy: 'keep-both-loaded',
    context_window: 8192,
  });
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.keyboard.press('Control+Shift+M');
  await dialog.getByRole('button', { name: 'Advanced', exact: true }).click();
  await expect(strategy).toHaveText('Keep models loaded');
});
