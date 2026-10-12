import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test.setTimeout(60_000);
test('settings opens the full wizard in the current hero draft scope and Cancel returns to its picker', async ({
  page,
}) => {
  await mockTauri(page, { onboarding_completed: true, ui_language: 'en', image_enabled: false });
  await page.goto('/');
  await page.getByRole('button', { name: 'New campaign' }).click();
  await page.getByRole('button', { name: 'Choose hero' }).click();
  await page.getByLabel('Hero name').fill('Scoped draft');
  await page.getByRole('button', { name: 'Settings', exact: true }).click();
  await page.getByRole('button', { name: 'Re-create character' }).click();
  await expect(page.locator('.dm-wizard')).toBeVisible();
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('.dm-wizard')).toHaveCount(0);
  await expect(page.getByLabel('Hero name')).toHaveValue('Scoped draft');
});

for (const lang of ['ru', 'en'] as const) {
  for (const width of [960, 1280]) {
    test(`prepared adventure ${lang} ${width}: source, scoped hero, real first scene`, async ({
      page,
    }) => {
      const height = width === 960 ? 640 : 800;
      await page.setViewportSize({ width, height });
      await mockTauri(page, { ui_language: lang, image_enabled: false });
      await page.goto('/');
      await page
        .getByRole('button', {
          name: lang === 'ru' ? 'Начать приключение' : 'Start adventure',
          exact: true,
        })
        .click();
      await page
        .getByRole('button', { name: lang === 'ru' ? 'Выбрать героя' : 'Choose hero', exact: true })
        .click();
      const enter = page.getByRole('button', {
        name: lang === 'ru' ? 'Войти в первую сцену' : 'Enter first scene',
      });
      await expect(enter).toBeVisible();
      const box = await enter.boundingBox();
      expect(box && box.y + box.height).toBeLessThanOrEqual(height);
      await expect(page.getByRole('radiogroup').getByRole('radio')).toHaveCount(4);
      await page.getByLabel(lang === 'ru' ? 'Имя героя' : 'Hero name').fill('Mira');
      await page.screenshot({
        path: `.artifacts/product-uiux/hero-choice-${lang}-${width}.png`,
        animations: 'disabled',
      });
      await enter.click();
      await expect(page.locator('.dm-vtt-panel')).toBeVisible();
      await expect(
        page.getByPlaceholder(lang === 'ru' ? 'Что ты делаешь?' : 'What do you do?'),
      ).toBeVisible();
      const prefs = await page.evaluate(() =>
        JSON.parse(localStorage.getItem('dmai-e2e-store:settings.json') ?? '{}'),
      );
      expect(prefs.pc.name).toBe('Mira');
      expect(prefs.pc.characterId).toMatch(/^[0-9a-f-]{36}$/);
      expect(prefs.active_campaign_id).toMatch(/^[0-9a-f-]{36}$/);
    });
  }
}

test('unknown create ACK survives reload and the hero draft survives Back and rejected confirmation', async ({
  page,
}) => {
  await mockTauri(
    page,
    { onboarding_completed: true, ui_language: 'en', image_enabled: false },
    { creationAckOnce: true, characterRejectOnce: true },
  );
  const creations: string[] = [];
  page.on('request', (request) => {
    if (new URL(request.url()).pathname === '/campaigns' && request.method() === 'POST')
      creations.push(request.postDataJSON().request_id);
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'New campaign' }).click();
  await page.getByRole('button', { name: 'Choose hero' }).click();
  await expect(page.getByRole('alert').filter({ hasText: 'Preparation' })).toBeVisible();
  await page.reload();
  await page.getByRole('button', { name: 'New campaign' }).click();
  await page.getByRole('button', { name: 'Choose hero' }).click();
  expect(creations).toHaveLength(2);
  expect(new Set(creations).size).toBe(1);
  await page.getByLabel('Hero name').fill('Persistent Mira');
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await page.getByRole('button', { name: 'Continue campaign' }).click();
  await expect(page.getByLabel('Hero name')).toHaveValue('Persistent Mira');
  await page.getByRole('button', { name: 'Enter first scene' }).click();
  await expect(
    page.getByRole('alert').filter({ hasText: 'Could not save the hero' }),
  ).toBeVisible();
  await page.reload();
  await expect(page.getByLabel('Hero name')).toHaveValue('Persistent Mira');
  await page.getByRole('button', { name: 'Enter first scene' }).click();
  await expect(page.locator('.dm-vtt-panel')).toBeVisible();
});
test('lost character ACK recovers the committed hero on cold reload without a second actor', async ({
  page,
}) => {
  await mockTauri(
    page,
    { onboarding_completed: true, ui_language: 'en', image_enabled: false },
    { characterAckOnce: true },
  );
  let writes = 0;
  page.on('request', (request) => {
    if (request.method() === 'PUT' && new URL(request.url()).pathname.endsWith('/character'))
      writes++;
  });
  await page.goto('/');
  await page.getByRole('button', { name: 'New campaign' }).click();
  await page.getByRole('button', { name: 'Choose hero' }).click();
  await page.getByLabel('Hero name').fill('Cold Mira');
  await page.getByRole('button', { name: 'Enter first scene' }).click();
  await expect(
    page.getByRole('alert').filter({ hasText: 'Could not save the hero' }),
  ).toBeVisible();
  await page.reload();
  await expect(page.locator('.dm-vtt-panel')).toBeVisible();
  expect(writes).toBe(1);
  const prefs = await page.evaluate(() =>
    JSON.parse(localStorage.getItem('dmai-e2e-store:settings.json') ?? '{}'),
  );
  expect(prefs.pc.name).toBe('Cold Mira');
});
