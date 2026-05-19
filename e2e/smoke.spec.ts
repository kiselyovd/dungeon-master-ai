import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.beforeEach(async ({ page }) => {
  // Pre-seed the persisted settings so neither the Onboarding modal nor
  // the initial CharacterWizard mounts: both cover the whole viewport and
  // would steal pointer events from the titlebar buttons below.
  await mockTauri(page, { onboarding_completed: true, hero_class: 'fighter' });
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
