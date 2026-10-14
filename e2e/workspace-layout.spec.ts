import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('versioned workspace restores panels, sizes, reading and view mode without losing a draft; reset keeps the campaign', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
    workspace_v1: {
      schemaVersion: 1,
      chatWidth: 544,
      contextWidth: 448,
      readingFontSize: 18,
      selectedContext: 'journal',
      expandedContext: false,
      viewMode: 'split',
    },
  });
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  await expect(page.getByRole('region', { name: "Bard's Journal" })).toBeVisible();
  const resize = page.getByRole('separator', { name: 'Resize context panel' });
  await expect(resize).toHaveAttribute('aria-valuenow', '448');
  await resize.focus();
  await page.keyboard.press('Shift+ArrowLeft');
  await expect(resize).toHaveAttribute('aria-valuenow', '512');
  const composer = page.getByPlaceholder('What do you do?');
  await composer.fill('Unsent workspace draft');
  const views = page.getByRole('group', { name: 'Workspace view' });
  await views.getByRole('button', { name: 'Chat', exact: true }).click();
  await expect(page.locator('.dm-vtt-panel')).toBeHidden();
  await expect(composer).toHaveValue('Unsent workspace draft');
  await page.reload();
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  await expect(page.locator('[data-table-layout]')).toHaveAttribute('data-view-mode', 'chat');
  await expect(composer).toHaveValue('Unsent workspace draft');
  await views.getByRole('button', { name: 'Split', exact: true }).click();
  await expect(resize).toHaveAttribute('aria-valuenow', '512');
  await expect(page.getByRole('combobox', { name: 'Reading text size' })).toContainText('18px');
  await page.getByRole('button', { name: 'Reset layout', exact: true }).click();
  await expect(page.locator('[data-table-layout]')).toHaveAttribute('data-view-mode', 'split');
  await expect(page.getByRole('region', { name: "Bard's Journal" })).toHaveCount(0);
  await expect(page.getByRole('combobox', { name: 'Reading text size' })).toContainText('17px');
  await expect(composer).toHaveValue('Unsent workspace draft');
  await expect(page.getByText('Fixture Chronicle', { exact: true }).first()).toBeVisible();
  await page.screenshot({
    path: '.artifacts/product-uiux/task-34-workspace-reset.png',
    animations: 'disabled',
  });
});

test('malformed workspace and unknown private fields recover to a usable split view', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
    chat_panel_width: 500,
    workspace_v1: {
      schemaVersion: 7,
      viewMode: 'private',
      selectedContext: 'foreign_actor',
      chatWidth: 90000,
      resume_proof: 'fixture-unwanted',
    },
  });
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  await expect(page.locator('[data-table-layout]')).toHaveAttribute('data-view-mode', 'split');
  await expect(page.getByPlaceholder('What do you do?')).toBeVisible();
  await expect(page.locator('.dm-vtt-panel')).toBeVisible();
});
