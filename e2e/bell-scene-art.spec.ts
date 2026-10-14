import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires isolated real backend and canonical actions.');
const locales = {
  ru: JSON.parse(
    readFileSync(new URL('../src/locales/ru/adventure.json', import.meta.url), 'utf8'),
  ),
  en: JSON.parse(
    readFileSync(new URL('../src/locales/en/adventure.json', import.meta.url), 'utf8'),
  ),
};
for (const language of ['ru', 'en'] as const)
  for (const width of [960, 1280, 1920]) {
    test(`public Bell paintings ${language} ${width}: six canonical transitions and real restore`, async ({
      page,
      request,
    }) => {
      const base = backend as string,
        locale = locales[language];
      await page.setViewportSize({
        width,
        height: width === 960 ? 640 : width === 1280 ? 800 : 1080,
      });
      const reduced = process.env.DMAI_E2E_REDUCED_MOTION !== '0';
      await page.emulateMedia({ reducedMotion: reduced ? 'reduce' : 'no-preference' });
      const created = await request.post(`${base}/campaigns`, {
        data: {
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          title: locale.title,
          premise: locale.premise,
          seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
        },
      });
      expect(created.ok()).toBe(true);
      const campaign = await created.json(),
        hero = randomUUID();
      const accepted = await request.put(`${base}/campaigns/${campaign.id}/character`, {
        data: {
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          session_id: campaign.last_session_id,
          character_id: hero,
          expected_revision: 1,
          causation_id: randomUUID(),
          character: {
            name: language === 'ru' ? 'Герой проверки' : 'Acceptance hero',
            heroClass: 'fighter',
            level: 1,
            hp: 12,
            hpMax: 12,
            ac: 14,
            abilities: { str: 15, dex: 14, con: 13, int: 12, wis: 10, cha: 8 },
            inventory: [],
          },
        },
      });
      expect(accepted.ok()).toBe(true);
      await mockTauri(page, {
        onboarding_completed: true,
        hero_class: 'fighter',
        ui_language: language,
        image_enabled: false,
        video_enabled: false,
        active_campaign_id: campaign.id,
        active_session_id: campaign.last_session_id,
      });
      await page.route(/^https?:\/\/[^/]+\/(?:campaigns|sessions|saves)(?:\/|$)/, async (route) => {
        const url = new URL(route.request().url());
        const reply = await route.fetch({ url: `${base}${url.pathname}${url.search}` });
        await route.fulfill({ response: reply });
      });
      await page.goto('/');
      await expect(page.locator('.dm-splash')).toHaveCount(0);
      const composer = page.getByPlaceholder(
        language === 'ru' ? 'Что ты делаешь?' : 'What do you do?',
      );
      await composer.fill(language === 'ru' ? 'Неотправленный черновик' : 'Unsent scene draft');
      const trigger = page
        .getByRole('button', { name: locale.open, exact: true })
        .and(page.locator('[aria-expanded]'));
      const inspect = async (location: string) => {
        await expect(page.locator('.dm-scene-transition')).toHaveCount(0);
        await expect(page.locator('.dm-video-disabled-toast')).toHaveCount(0);
        await expect(composer).toHaveValue(
          language === 'ru' ? 'Неотправленный черновик' : 'Unsent scene draft',
        );
        const id = `scene.bell.${location}.v2`,
          title = locale.locations[location];
        const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
        expect(current.projection.adventure.scene_asset_id).toBe(id);
        const preview = page.getByRole('button', {
          name: locale.scene_open.replace('{{location}}', title),
          exact: true,
        });
        await expect(preview).toBeVisible();
        if (location === 'archive')
          await expect(page.getByTestId('current-scene-art')).toHaveCount(0);
        else {
          const stage = page.getByTestId('current-scene-art');
          await expect(stage).toHaveAttribute('data-location', location);
          await expect
            .poll(() =>
              stage
                .locator('img')
                .evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth),
            )
            .toBe(1440);
          await expect(page.locator('.dm-vtt-controls')).toBeHidden();
          await expect(page.locator('.dm-vtt-scale')).toBeHidden();
        }
        await preview.click();
        const modal = page.getByRole('dialog', { name: title, exact: true });
        await expect(modal.getByTestId('scene-art-preview')).toHaveAttribute(
          'data-location',
          location,
        );
        await expect(modal.locator('img')).toHaveAttribute('data-asset-id', id);
        await expect(modal.locator('img')).toHaveCSS('object-fit', 'contain');
        if (reduced)
          expect(
            await modal.evaluate((node) =>
              node
                .getAnimations({ subtree: true })
                .some((animation) => animation.playState === 'running'),
            ),
          ).toBe(false);
        await page.screenshot({
          path: `.artifacts/visual-production/bell-scene-${location}-${language}-${width}.png`,
          animations: 'disabled',
        });
        await modal.getByRole('button', { name: locale.scene_close, exact: true }).click();
      };
      await inspect('town');
      for (const location of ['shrine', 'town', 'warehouse', 'town', 'quay', 'archive', 'belfry']) {
        await trigger.click();
        await page
          .getByRole('tab', { name: language === 'ru' ? 'Сцена' : 'Scene', exact: true })
          .click();
        const label = locale.actions.move.replace('{{location}}', locale.locations[location]);
        const response = page.waitForResponse(
          (reply) =>
            reply.url().endsWith('/adventure/actions') && reply.request().method() === 'POST',
        );
        await page.getByRole('button', { name: label, exact: true }).click();
        expect((await response).status()).toBe(200);
        await page.keyboard.press('Escape');
        await inspect(location);
      }
      const before = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
      const saved = await request.post(`${base}/sessions/${before.session_id}/saves`, {
        data: {
          kind: 'manual',
          title: language === 'ru' ? 'Звонница: проверка восстановления' : 'Belfry restore check',
          summary: '',
          tag: 'exploration',
          campaign_id: campaign.id,
          expected_revision: before.campaign.revision,
          character: before.projection.character,
          causation_id: randomUUID(),
        },
      });
      expect(saved.ok()).toBe(true);
      const save = await saved.json();
      const now = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
      const moved = await request.post(`${base}/campaigns/${campaign.id}/adventure/actions`, {
        data: {
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          session_id: now.session_id,
          character_id: hero,
          expected_revision: now.campaign.revision,
          causation_id: randomUUID(),
          action: { kind: 'move', location: 'archive' },
        },
      });
      expect(moved.ok()).toBe(true);
      const changed = await moved.json();
      const restored = await request.post(`${base}/saves/${save.id}/restore`, {
        data: {
          campaign_id: campaign.id,
          session_id: changed.session_id,
          expected_revision: changed.campaign.revision,
          causation_id: randomUUID(),
        },
      });
      expect(restored.ok()).toBe(true);
      await page.reload();
      await inspect('belfry');
      await page.route('**/*bell-scene-belfry-v2*.webp', (route) => route.abort());
      await page.reload();
      await expect(
        page.getByTestId('current-scene-art').getByText(locale.scene_unavailable),
      ).toBeVisible();
      await page
        .getByRole('button', {
          name: locale.scene_open.replace('{{location}}', locale.locations.belfry),
          exact: true,
        })
        .click();
      const unavailable = page.getByRole('dialog', { name: locale.locations.belfry, exact: true });
      await expect(unavailable.getByText(locale.scene_unavailable)).toBeVisible();
      await unavailable.getByRole('button', { name: locale.scene_close, exact: true }).click();
      await expect(page.getByRole('dialog')).toHaveCount(0);
    });
  }
