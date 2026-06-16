import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * First-run onboarding (redesigned flow): welcome -> preset -> hero -> chat ->
 * image -> video. The "Skip setup" (manual) preset trims the flow to
 * welcome -> preset -> hero, so it is fully completable in e2e without any
 * backend (chat/image steps that need downloads/providers are skipped).
 *
 * No `onboarding_completed` seed here so the modal mounts on load.
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page);
});

test('mounts on first run and shows the welcome step', async ({ page }) => {
  await page.goto('/');
  // The step counter (e.g. "Step 1 of 5") is unique to the onboarding modal.
  await expect(page.getByText(/Step 1 of 5/i)).toBeVisible();
  await expect(page.getByRole('button', { name: /Continue/i })).toBeVisible();
});

test('Continue advances Welcome -> Preset (hero precedes the technical steps)', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Continue/i }).click();
  // Preset step heading.
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(/Choose a preset/i);
  await expect(page.getByText(/Step 2 of 5/i)).toBeVisible();
  // The stepper lists "Create hero" before the technical steps (redesign).
  await expect(page.getByText('Create hero', { exact: true })).toBeVisible();
});

test('"Skip setup" preset trims the flow to welcome -> preset -> hero', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Continue/i }).click();
  // Select the manual/"Skip setup" preset card (role=radio).
  await page.getByRole('radio', { name: /Skip setup/i }).click();
  // Counter now reflects the 3-step manual flow.
  await expect(page.getByText(/Step 2 of 3/i)).toBeVisible();
  await page.getByRole('button', { name: /Continue/i }).click();
  // Lands directly on the Hero step (no chat/image).
  await expect(page.getByText(/Step 3 of 3/i)).toBeVisible();
  await expect(page.getByText(/Build from scratch/i)).toBeVisible();
});

test('completing the manual flow by picking a class dismisses onboarding', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /Continue/i }).click();
  await page.getByRole('radio', { name: /Skip setup/i }).click();
  await page.getByRole('button', { name: /Continue/i }).click();
  // Hero step: pick a class card -> finalize. The Fighter card is a button.
  await page.getByRole('button', { name: /Fighter/i }).click();
  // Onboarding modal unmounts once completed (step counter gone).
  await expect(page.getByText(/Step \d of \d/i)).toHaveCount(0);
});

test('language picker flips EN/RU', async ({ page }) => {
  await page.goto('/');
  const ru = page.getByRole('button', { name: 'RU', exact: true });
  const en = page.getByRole('button', { name: 'EN', exact: true });
  await expect(en).toHaveAttribute('aria-pressed', 'true');
  await ru.click();
  await expect(ru).toHaveAttribute('aria-pressed', 'true');
  await expect(en).toHaveAttribute('aria-pressed', 'false');
});

test('onboarding does NOT reappear once completed (hydration gate)', async ({ page }) => {
  // Seed the persisted completed flag: the redesigned hydration gate must keep
  // the modal from flashing on a relaunch. (Audit blocker 1.)
  await mockTauri(page, { onboarding_completed: true, hero_class: 'fighter' });
  await page.goto('/');
  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  // The onboarding step counter must be absent (modal never mounted).
  await expect(page.getByText(/Step \d of \d/i)).toHaveCount(0);
});
