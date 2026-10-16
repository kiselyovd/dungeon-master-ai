import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * App shell: titlebar panels (Saves / Journal / NPCs), status-bar chips that
 * deep-link into Settings tabs, window controls, and keyboard shortcuts.
 *
 * Seeds skip onboarding + the initial wizard + the blocking preflight modal so
 * the titlebar is interactive.
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
    ui_language: 'en',
  });
  // SavesScreen fetches the saves list on open; return an empty list. The
  // pattern is a tight regex (not `**/saves**`) so it does NOT swallow vite's
  // own source modules whose path contains "saves" (e.g. /src/state/saves.ts) -
  // that would replace the JS with `[]` and blank the whole app.
  await page.route(
    /^http:\/\/127\.0\.0\.1:31415\/(?:sessions\/[^/]+\/)?saves(?:\/|\?|$)/,
    async (route) => {
      await route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
    },
  );
});

test('Journal button opens and closes the journal context panel', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Journal', exact: true }).click();
  const panel = page.getByRole('tabpanel', { name: 'Journal', exact: true });
  await expect(panel.getByRole('region', { name: "Bard's Journal", exact: true })).toBeVisible();
  await page
    .getByRole('complementary', { name: 'Adventure', exact: true })
    .getByRole('button', { name: 'Close', exact: true })
    .click();
  await expect(panel).toHaveCount(0);
});

test('NPCs button opens and closes the NPC context panel', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'NPCs', exact: true }).click();
  const panel = page.getByRole('tabpanel', { name: 'NPCs', exact: true });
  await expect(panel.getByRole('region', { name: 'NPC Memory', exact: true })).toBeVisible();
  await page
    .getByRole('complementary', { name: 'Adventure', exact: true })
    .getByRole('button', { name: 'Close', exact: true })
    .click();
  await expect(panel).toHaveCount(0);
});

test('Saves button opens the saves dialog', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Saves', exact: true }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
});

test('Ctrl+Shift+S opens the saves dialog', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Saves', exact: true })).toBeVisible();
  await page.keyboard.press('Control+Shift+S');
  await expect(page.getByRole('dialog')).toBeVisible();
});

test('status bar provider chip deep-links into Settings (Chat tab selected)', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Dungeon Master settings', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('tab', { name: 'Chat', exact: true })).toHaveAttribute(
    'aria-selected',
    'true',
  );
});

test('window controls are present', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Minimize', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Maximize', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Close', exact: true })).toBeVisible();
});
