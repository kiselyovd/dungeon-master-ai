import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('failed Save preserves the draft; Cancel discards it; ACK commits all preferences', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'en',
    active_provider: 'local-mistralrs',
  });
  let reject = true;
  await page.route('**/settings/v2', async (route) =>
    route.fulfill({
      status: reject ? 503 : 200,
      contentType: 'application/json',
      body: reject ? '{"error":{"code":"unavailable","message":"Fixture unavailable"}}' : '{}',
    }),
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByLabel(/Provider/i).click();
  await dialog.getByRole('option', { name: 'Model server', exact: true }).click();
  await dialog.getByLabel(/API key/i).fill('fixture-key');
  await dialog.getByPlaceholder('qwen3-1.7b').fill('model-a');
  await dialog.getByRole('tab', { name: 'Image', exact: true }).click();
  const images = dialog.getByRole('checkbox', { name: /Enable/i });
  await images.uncheck();
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(dialog.getByTestId('settings-save-error')).toBeVisible();
  await dialog.getByRole('tab', { name: 'Chat', exact: true }).click();
  await expect(dialog.getByPlaceholder('qwen3-1.7b')).toHaveValue('model-a');
  await dialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await dialog.getByRole('tab', { name: 'Image', exact: true }).click();
  await expect(images).toBeChecked();
  await images.uncheck();
  await dialog.getByRole('tab', { name: 'Chat', exact: true }).click();
  await dialog.getByLabel(/Provider/i).click();
  await dialog.getByRole('option', { name: 'Model server', exact: true }).click();
  await dialog.getByLabel(/API key/i).fill('fixture-key');
  await dialog.getByPlaceholder('qwen3-1.7b').fill('model-a');
  reject = false;
  await dialog.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await expect(dialog.getByPlaceholder('qwen3-1.7b')).toHaveValue('model-a');
  await dialog.getByRole('tab', { name: 'Image', exact: true }).click();
  await expect(images).not.toBeChecked();
});
