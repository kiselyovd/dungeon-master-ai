import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * Settings modal: tab navigation, the unified provider block (local config
 * inline in the Chat tab vs. cloud fields), behavior toggles, and save/cancel.
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });
  await page.route('**/local/runtime/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ llm: { state: 'off' }, image: { state: 'off' } }),
    });
  });
  await page.route('**/local-llm/manifest', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{"models":[]}' });
  });
  await page.route('**/settings/v2', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
});

async function openSettings(page: import('@playwright/test').Page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

test('exposes all five tabs and they switch', async ({ page }) => {
  await openSettings(page);
  const dialog = page.getByRole('dialog');
  for (const name of ['Chat', 'Local LLM', 'Image', 'Video', 'Behavior']) {
    const tab = dialog.getByRole('tab', { name, exact: true });
    await expect(tab).toBeVisible();
    await tab.click();
    await expect(tab).toHaveAttribute('aria-selected', 'true');
  }
});

test('unified provider block: local hides Base URL, OpenAI-compat shows it', async ({ page }) => {
  await openSettings(page);
  const dialog = page.getByRole('dialog');
  // Seeded provider is local-mistralrs -> no cloud Base URL field, inline local
  // config instead.
  await expect(dialog.getByLabel(/Base URL/i)).toHaveCount(0);
  // Switch the provider to OpenAI-compatible -> cloud fields appear.
  await dialog.getByLabel(/Provider/i).selectOption('openai-compat');
  await expect(dialog.getByLabel(/Base URL/i)).toBeVisible();
  await expect(dialog.getByLabel(/API key/i)).toBeVisible();
  // Model is a ModelSelector (combobox), targeted by its placeholder.
  await expect(dialog.getByPlaceholder('qwen3-1.7b')).toBeVisible();
});

test('Behavior tab shows the scene-transitions toggle', async ({ page }) => {
  await openSettings(page);
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('tab', { name: 'Behavior', exact: true }).click();
  await expect(dialog.getByText(/Scene transitions/i)).toBeVisible();
});

test('Cancel closes the modal without saving', async ({ page }) => {
  await openSettings(page);
  await page.getByRole('button', { name: /Cancel/i }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('Save persists and closes (POST /settings mocked 200)', async ({ page }) => {
  let posted = false;
  await page.route('**/settings/v2', async (route) => {
    if (route.request().method() === 'POST') posted = true;
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
  await openSettings(page);
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel(/Provider/i).selectOption('openai-compat');
  await dialog.getByLabel(/Base URL/i).fill('https://openrouter.ai/api/v1');
  await dialog.getByLabel(/API key/i).fill('sk-test-key');
  await dialog.getByPlaceholder('qwen3-1.7b').fill('anthropic/claude-3.5-sonnet');
  await page.getByRole('button', { name: /^Save$/i }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(posted).toBe(true);
});
