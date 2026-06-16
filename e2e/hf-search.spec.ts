import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * Hugging Face model search inside Settings -> Local LLM. Validates the wire
 * contract fix (audit blocker 2): the backend returns `repo_id` / `last_modified`
 * / `siblings[].filename`, and the result cards must render the repo id (it was
 * `undefined` before the fix, so every card came up blank).
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
  });
  await page.route('**/hf/token/status', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ connected: false }),
    });
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
  // Backend wire shape (post-fix): repo_id / last_modified / siblings[].filename.
  await page.route('**/hf/search**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        {
          repo_id: 'Qwen/Qwen3-4B-GGUF',
          likes: 120,
          downloads: 50000,
          gated: false,
          tags: ['qwen3', 'text-generation'],
          last_modified: '2026-05-01T00:00:00.000Z',
          siblings: [{ filename: 'qwen3-4b-q4_k_m.gguf', size: 2_300_000_000 }],
        },
        {
          repo_id: 'meta-llama/Llama-3-8B',
          likes: 999,
          downloads: 800000,
          gated: true,
          tags: ['llama3'],
          last_modified: '2026-04-01T00:00:00.000Z',
          siblings: [],
        },
      ]),
    });
  });
});

test('HF search renders result cards with the repo id (blocker 2 fix)', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('tab', { name: 'Local LLM', exact: true }).click();

  // Expand the "Search Hugging Face" collapsible (collapsed by default).
  await dialog.getByRole('button', { name: /Search Hugging Face/i }).click();

  // Type a query and submit.
  await dialog.getByPlaceholder(/Search HuggingFace/i).fill('qwen3');
  await dialog.getByRole('button', { name: 'Search', exact: true }).click();

  // repo_id must render (it was undefined before the wire-contract fix).
  await expect(dialog.getByText('Qwen/Qwen3-4B-GGUF')).toBeVisible();
  await expect(dialog.getByText('meta-llama/Llama-3-8B')).toBeVisible();

  // The compatible Qwen result offers Download; the gated Llama offers Open HF.
  await expect(dialog.getByTestId('hf-card-compatible')).toBeVisible();
  await expect(dialog.getByTestId('hf-card-gated')).toBeVisible();
  // Download button scoped to the compatible card (ManageDownloads also has some).
  await expect(
    dialog.getByTestId('hf-card-compatible').getByRole('button', { name: /Download/i }),
  ).toBeVisible();
});
