import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('production library previews and cancels a portable import without replacing the current campaign', async ({
  page,
}) => {
  await mockTauri(page, { onboarding_completed: true, ui_language: 'ru' });
  await page.addInitScript(() => {
    Object.defineProperty(window, 'isTauri', { value: true });
    const native = (
      window as unknown as {
        __TAURI_INTERNALS__: { invoke(command: string, args?: unknown): Promise<unknown> };
      }
    ).__TAURI_INTERNALS__;
    const original = native.invoke.bind(native);
    (window as unknown as { bundleCommits: number }).bundleCommits = 0;
    native.invoke = async (command, args) => {
      if (command === 'desktop_bundle_pick_import')
        return {
          operationId: '00000000-0000-4000-8000-000000000001',
          fileName: 'Поход.dmai',
          preview: {
            kind: 'full_campaign_backup',
            title: 'Импортируемый поход',
            character_count: 2,
            message_count: 12,
            media_count: 1,
            private_backup: true,
          },
        };
      if (command === 'desktop_bundle_cancel') return null;
      if (command === 'desktop_bundle_confirm_import') {
        (window as unknown as { bundleCommits: number }).bundleCommits++;
        return {
          campaign_id: '00000000-0000-4000-8000-000000000002',
          session_id: '00000000-0000-4000-8000-000000000003',
          revision: 1,
          remapped_ids: {},
        };
      }
      return original(command, args);
    };
  });
  await page.goto('/');
  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  await page.getByRole('button', { name: 'Импорт кампании', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Импорт кампании' });
  await expect(dialog.getByText('Импортируемый поход')).toBeVisible();
  await expect(dialog.getByText('Персонажей: 2 · Сообщений: 12 · Изображений: 1')).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as { bundleCommits: number }).bundleCommits),
  ).toBe(0);
  await dialog.getByRole('button', { name: 'Отмена' }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as { bundleCommits: number }).bundleCommits),
  ).toBe(0);
  await page.getByRole('button', { name: 'Импорт кампании', exact: true }).click();
  await dialog.getByRole('button', { name: 'Создать новую кампанию' }).click();
  await expect(
    dialog.getByText('Новая кампания создана. Откройте её, когда будете готовы.'),
  ).toBeVisible();
  expect(
    await page.evaluate(() => (window as unknown as { bundleCommits: number }).bundleCommits),
  ).toBe(1);
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  await page.screenshot({
    path: '.artifacts/product-uiux/task-41-browser-import.png',
    animations: 'disabled',
  });
});
