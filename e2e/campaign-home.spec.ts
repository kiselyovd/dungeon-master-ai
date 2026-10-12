import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const campaign = {
  id: '62ba8579-904b-4460-a68a-df6268ed404a',
  schema_version: 1,
  ruleset_id: 'srd-5.2.1',
  title: 'Серебряная долина',
  premise: 'Древняя дорога снова зовёт героев.',
  last_session_id: 'b5271021-b82c-40c4-aa9f-554af0d79b22',
  revision: 4,
  updated_at: '2026-08-13T03:21:00.000Z',
};

test('empty campaign library owns the Russian production root and opens creation', async ({
  page,
}) => {
  await mockTauri(page, { onboarding_completed: true, ui_language: 'ru' });
  await page.goto('/');

  await expect(page.getByText('DUNGEON MASTER AI')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
  await expect(page.locator('.dm-vtt-panel')).toHaveCount(0);
  await expect(page.getByPlaceholder('Что ты делаешь?')).toHaveCount(0);
  await expect(page.getByText(/Anthropic|Claude|join room|создать комнату/i)).toHaveCount(0);

  await page.getByRole('button', { name: 'Новая кампания' }).click();
  await expect(page.getByRole('heading', { name: 'С чего начнётся история?' })).toBeVisible();
  await expect(page.getByRole('radio', { name: /Готовое приключение/ })).toBeChecked();

  await page.getByRole('button', { name: 'Назад' }).click();
  await page.getByRole('button', { name: 'EN' }).click();
  await expect(page.getByRole('heading', { name: 'Your adventures' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'New campaign' })).toBeVisible();
});

test('latest campaign is primary and continue uses its backend identity', async ({ page }) => {
  await mockTauri(page, { onboarding_completed: true, ui_language: 'ru' });
  await page.route(/^https?:\/\/[^/]+\/campaigns(?:\/[^/?]+)?(?:\?|$)/, async (route) => {
    const isDetail = new URL(route.request().url()).pathname !== '/campaigns';
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(
        isDetail
          ? {
              ...campaign,
              seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
              created_at: '2026-08-13T03:21:00.000Z',
            }
          : [campaign],
      ),
    });
  });
  await page.route('**/campaigns/*/state', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        campaign: {
          ...campaign,
          seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
          created_at: '2026-08-13T03:21:00.000Z',
        },
        session_id: campaign.last_session_id,
        character_id: null,
        projection: {
          character: {},
          messages: [],
          combat: null,
          scene: null,
          journal: [],
          npcs: [],
        },
      }),
    });
  });
  await page.goto('/');

  await expect(page.getByRole('heading', { name: campaign.title })).toBeVisible();
  await page.getByRole('button', { name: 'Продолжить кампанию' }).click();
  await expect(page.getByRole('heading', { name: 'Герой этого приключения' })).toBeVisible();
  await page.getByRole('button', { name: 'Создать своего героя' }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('tablist')).toBeVisible();
});
