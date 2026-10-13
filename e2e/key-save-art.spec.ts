import { expect, type Locator, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

async function decoded(image: Locator, width: number) {
  await expect
    .poll(() => image.evaluate((node: HTMLImageElement) => node.complete && node.naturalWidth))
    .toBe(width);
}
const sizes = [
  { width: 960, height: 640 },
  { width: 1280, height: 800 },
  { width: 1920, height: 1080 },
];
const campaignId = '2c7d0d90-e970-47a7-aac4-66a147f96e58';
const tags = ['combat', 'exploration', 'dialog', 'npc'];
for (const language of ['ru', 'en'])
  for (const viewport of sizes) {
    test(`key art ${language} ${viewport.width}: welcome crop and real first action`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      await mockTauri(page, { ui_language: language, image_enabled: false });
      await page.goto('/');
      await expect(page.locator('.dm-splash')).toHaveCount(0);
      const art = page.getByTestId('onboarding-hero-art');
      await decoded(art, 1440);
      await expect(art).toHaveAttribute('data-asset-id', 'onboarding.v2');
      const start = page.getByRole('button', {
        name: language === 'ru' ? 'Начать приключение' : 'Start adventure',
        exact: true,
      });
      await expect(start).toBeVisible();
      const box = await start.boundingBox();
      expect(box && box.y + box.height).toBeLessThanOrEqual(viewport.height);
      await page.screenshot({
        path: `.artifacts/visual-production/welcome-${language}-${viewport.width}.png`,
        animations: 'disabled',
      });
      await start.click();
      await expect(
        page.getByRole('heading', {
          name: language === 'ru' ? 'С чего начнётся история?' : 'Where does the story begin?',
        }),
      ).toBeVisible();
    });
    test(`save art ${language} ${viewport.width}: all four categories with actual long snapshot text`, async ({
      page,
    }) => {
      await page.setViewportSize(viewport);
      await page.emulateMedia({ reducedMotion: 'reduce' });
      await mockTauri(page, {
        onboarding_completed: true,
        hero_class: 'fighter',
        ui_language: language,
        image_enabled: false,
      });
      const saves = tags.map((tag, index) => ({
        id: `save-${tag}`,
        campaign_id: campaignId,
        session_id: '53a7fa02-cfb6-4717-b7fe-e6408f81a569',
        snapshot_id: `snapshot-${tag}`,
        revision: index + 3,
        kind: 'manual',
        tag,
        title:
          language === 'ru'
            ? `Сохранение ${index + 1}: долгая дорога через Серебряную долину к старому каменному мосту`
            : `Save ${index + 1}: the long road through Silver Valley towards the old stone bridge`,
        summary:
          language === 'ru'
            ? `Подтверждённый снимок ${index + 1}: герой остановился на дороге и сохранил текущий прогресс. Изображение не меняет эти сведения.`
            : `Confirmed snapshot ${index + 1}: the hero paused on the road and saved the current progress. The category painting does not change these details.`,
        created_at: '2026-10-13T00:00:00Z',
        turn_number: index + 3,
      }));
      await page.route('**/sessions/**', (route) =>
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify(route.request().url().includes('/saves') ? saves : { messages: [] }),
        }),
      );
      let healthy = false;
      await page.route('**/health', (route) =>
        route.fulfill({ status: healthy ? 200 : 503, contentType: 'application/json', body: '{}' }),
      );
      await page.goto('/');
      const splash = page.locator('.dm-splash-image');
      await decoded(splash, 1440);
      await expect(splash).toHaveAttribute('data-asset-id', 'splash.v2');
      await page.screenshot({
        path: `.artifacts/visual-production/splash-${language}-${viewport.width}.png`,
        animations: 'disabled',
      });
      healthy = true;
      await expect(page.locator('.dm-splash')).toHaveCount(0);
      const peaceful = page.locator('.dm-vtt-empty-art');
      await decoded(peaceful, 1440);
      await expect(peaceful).toHaveAttribute('data-asset-id', 'vtt.empty.v2');
      await page
        .getByRole('button', { name: language === 'ru' ? 'Сейвы' : 'Saves', exact: true })
        .click();
      for (const [index, tag] of tags.entries()) {
        const row = page.locator(`[data-save-id="save-${tag}"]`);
        await row.click();
        const detail = page.locator('.dm-save-detail');
        const image = detail.locator('img');
        await decoded(image, 1024);
        await expect(image).toHaveAttribute('data-asset-id', `save.${tag}.v2`);
        await expect(image).toHaveCSS('object-fit', 'contain');
        await expect(detail.getByRole('heading')).toHaveText(saves[index]?.title ?? 'missing');
        await expect(detail.locator('.dm-save-detail-summary p')).toHaveText(
          saves[index]?.summary ?? 'missing',
        );
        const load = detail.getByRole('button', {
          name: language === 'ru' ? 'Загрузить' : 'Load',
          exact: true,
        });
        await load.scrollIntoViewIfNeeded();
        await expect(load).toBeVisible();
        const box = await load.boundingBox();
        expect(box && box.y + box.height).toBeLessThanOrEqual(viewport.height);
        await page.screenshot({
          path: `.artifacts/visual-production/save-${tag}-${language}-${viewport.width}.png`,
          animations: 'disabled',
        });
      }
      const motion = await page.locator('.dm-saves-tome').evaluate((node) => ({
        durations: getComputedStyle(node)
          .animationDuration.split(',')
          .map((value) => parseFloat(value)),
        iterations: getComputedStyle(node).animationIterationCount,
        running: node.getAnimations().some((animation) => animation.playState === 'running'),
      }));
      expect(motion.durations.every((duration) => duration <= 0.0001)).toBe(true);
      expect(motion.iterations).toBe('1');
      expect(motion.running).toBe(false);
    });
  }

for (const viewport of sizes)
  test(`library paintings ${viewport.width}: empty and populated long Russian title`, async ({
    page,
  }) => {
    await page.setViewportSize(viewport);
    await mockTauri(page, { onboarding_completed: true, ui_language: 'ru' });
    let populated = false;
    const campaign = {
      id: campaignId,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title:
        'Долгая история Серебряной долины: путешествие от старого тракта до далёкой каменной переправы',
      premise:
        'Подтверждённое описание приключения остаётся читаемым рядом с общей атмосферной картиной.',
      last_session_id: '53a7fa02-cfb6-4717-b7fe-e6408f81a569',
      revision: 7,
      updated_at: '2026-10-13T00:00:00Z',
    };
    await page.route('**/campaigns', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(populated ? [campaign] : []),
      }),
    );
    await page.goto('/');
    await expect(page.getByRole('heading', { name: 'Ваши приключения' })).toBeVisible();
    await decoded(page.locator('main img[data-asset-id="onboarding.v2"]'), 1440);
    await expect(page.getByRole('button', { name: 'Новая кампания', exact: true })).toBeVisible();
    await page.screenshot({
      path: `.artifacts/visual-production/library-empty-${viewport.width}.png`,
      animations: 'disabled',
    });
    populated = true;
    await page.reload();
    await decoded(page.locator('main img[data-asset-id="vtt.empty.v2"]'), 1440);
    await expect(page.getByRole('heading', { name: campaign.title })).toBeVisible();
    const resume = page.getByRole('button', { name: 'Продолжить кампанию', exact: true });
    await resume.scrollIntoViewIfNeeded();
    await expect(resume).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: `.artifacts/visual-production/library-populated-${viewport.width}.png`,
      animations: 'disabled',
    });
  });

test('missing key images cannot delay healthy startup or block the first action', async ({
  page,
}) => {
  await mockTauri(page, { ui_language: 'ru', image_enabled: false });
  await page.route('**/*key-*-v2*.webp', (route) => route.abort());
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  await expect(page.getByTestId('onboarding-hero-art')).toHaveCSS('visibility', 'hidden');
  await page.getByRole('button', { name: 'Начать приключение', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'С чего начнётся история?' })).toBeVisible();
});

test('missing splash and exploration painting leave the initialized table usable', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    ui_language: 'ru',
    image_enabled: false,
  });
  await page.route('**/*key-*-v2*.webp', (route) => route.abort());
  await page.route('**/sessions/**', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' }),
  );
  await page.goto('/');
  await expect(page.locator('.dm-splash')).toHaveCount(0);
  await expect(page.locator('.dm-vtt-empty-art')).toHaveCSS('visibility', 'hidden');
  const draft = page.getByPlaceholder('Что ты делаешь?');
  await draft.fill('Продолжить осмотр дороги');
  await expect(draft).toHaveValue('Продолжить осмотр дороги');
});
