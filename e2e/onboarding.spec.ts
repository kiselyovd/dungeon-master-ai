import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

/**
 * First-run onboarding records adventure or guest intent. Adventure source
 * and hero selection happen in their campaign scope before model setup.
 *
 * No `onboarding_completed` seed here so the modal mounts on load.
 */
test.beforeEach(async ({ page }) => {
  await mockTauri(page, { ui_language: 'en' });
});

test('mounts on first run with adventure and guest choices', async ({ page }) => {
  await page.goto('/');
  const welcome = page.getByRole('dialog', { name: 'Your next adventure', exact: true });
  await expect(welcome).toBeVisible();
  await expect(welcome.getByRole('button', { name: 'Start adventure', exact: true })).toBeVisible();
  await expect(welcome.getByRole('button', { name: 'Join game', exact: true })).toBeVisible();
  await expect(page.getByTestId('onboarding-hero-art')).toBeVisible();
});

test('Start adventure opens the campaign source choice before technical setup', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Start adventure', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Where does the story begin?' })).toBeVisible();
  await expect(page.getByRole('radio', { name: /Prepared adventure/ })).toBeChecked();
  await expect(page.getByRole('button', { name: 'Choose hero', exact: true })).toBeVisible();
  await expect(page.getByRole('dialog', { name: 'Your next adventure', exact: true })).toHaveCount(
    0,
  );
});

test('prepared adventure reaches scoped hero choice without requiring model setup', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Start adventure', exact: true }).click();
  await page.getByRole('button', { name: 'Choose hero', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'Hero for this adventure', exact: true }),
  ).toBeVisible();
  await expect(page.getByRole('radiogroup').getByRole('radio')).toHaveCount(4);
  await expect(
    page.getByRole('button', { name: 'Create your own hero', exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel('Hero name', { exact: true })).toBeVisible();
  await expect(page.getByLabel('API key', { exact: true })).toHaveCount(0);
});

test('reduced motion keeps welcome art visible without ambient animation', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');

  const art = page.getByTestId('onboarding-hero-art');
  await expect(art).toBeVisible();
  await expect(page.locator('.dm-onboarding-hero')).toHaveCSS('animation-name', 'none');
});

test('entering with a prepared class completes onboarding and opens the production table', async ({
  page,
}) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Start adventure', exact: true }).click();
  await page.getByRole('button', { name: 'Choose hero', exact: true }).click();
  await page.getByRole('radio', { name: /^Fighter / }).check();
  await page.getByRole('button', { name: 'Enter first scene', exact: true }).click();
  await expect(page.locator('.dm-vtt-panel')).toBeVisible();
  await expect(page.locator('.dm-char-fab')).toContainText('Fighter');
  await expect(page.getByRole('dialog', { name: 'Your next adventure', exact: true })).toHaveCount(
    0,
  );
});

test('language picker flips EN/RU', async ({ page }) => {
  await page.goto('/');
  const ru = page.getByRole('button', { name: 'РУ', exact: true });
  const en = page.getByRole('button', { name: 'EN', exact: true });
  await expect(en).toHaveAttribute('aria-pressed', 'true');
  await ru.click();
  await expect(ru).toHaveAttribute('aria-pressed', 'true');
  await expect(en).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByRole('button', { name: 'Начать приключение', exact: true })).toBeVisible();
});

test('onboarding does NOT reappear once completed (hydration gate)', async ({ page }) => {
  // Seed the persisted completed flag: the redesigned hydration gate must keep
  // the modal from flashing on a relaunch. (Audit blocker 1.)
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
  });
  await page.goto('/');
  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  await expect(page.getByRole('dialog', { name: 'Your next adventure', exact: true })).toHaveCount(
    0,
  );
  await expect(page.locator('.dm-vtt-panel')).toBeVisible();
});
