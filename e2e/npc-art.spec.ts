import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const campaignId = '2c7d0d90-e970-47a7-aac4-66a147f96e58';
const archetypes = [
  'innkeeper',
  'guard',
  'merchant',
  'rogue',
  'mage',
  'priestess',
  'knight',
  'peasant',
];
const roles = {
  en: ['Innkeeper', 'Guard', 'Merchant', 'Rogue', 'Mage', 'Priestess', 'Knight', 'Peasant'],
  ru: ['Трактирщик', 'Страж', 'Купец', 'Плут', 'Волшебница', 'Жрица', 'Рыцарь', 'Крестьянин'],
};
for (const lang of ['en', 'ru'] as const)
  for (const width of [960, 1280, 1920]) {
    test(`NPC portraits ${lang} ${width}: eight full views, legacy identity and cold restore`, async ({
      page,
    }) => {
      await page.setViewportSize({
        width,
        height: width === 960 ? 640 : width === 1280 ? 800 : 1080,
      });
      const records = archetypes.map((archetype, index) => ({
        id: `generic-${archetype}`,
        campaign_id: campaignId,
        name:
          index >= 6
            ? lang === 'ru'
              ? 'Мира'
              : 'Mira'
            : `${lang === 'ru' ? 'Знакомый' : 'Contact'} ${index + 1}`,
        role: roles[lang][index],
        disposition: 'neutral',
        trust: 0,
        facts: [],
        updated_at: '2026-10-13T00:00:00Z',
      }));
      records.push({
        id: 'known-person',
        campaign_id: campaignId,
        name: 'Known face',
        role: 'Mage',
        disposition: 'neutral',
        trust: 0,
        facts: [],
        updated_at: '2026-10-13T00:00:00Z',
      });
      await mockTauri(page, {
        onboarding_completed: true,
        hero_class: 'fighter',
        ui_language: lang,
        public_npcs: records,
      });
      await page.addInitScript(
        ({ campaignId }) => {
          const key = 'dmai-e2e-store:npc-appearance.json';
          if (!localStorage.getItem(key))
            localStorage.setItem(
              key,
              JSON.stringify({
                assignments: { [`${campaignId}:known-person`]: 'npc.guard.legacy.v1' },
              }),
            );
        },
        { campaignId },
      );
      await page.goto('/');
      await page.getByRole('button', { name: lang === 'en' ? 'NPCs' : 'NPC', exact: true }).click();
      await expect(page.locator('img[data-asset-id="npc.guard.legacy.v1"]')).toHaveCount(1);
      for (const archetype of archetypes) {
        const thumb = page.locator(`img[data-asset-id="npc.${archetype}.v2"]`).first();
        await expect(thumb, `missing NPC archetype ${archetype}`).toHaveCount(1, { timeout: 2000 });
        await expect
          .poll(() =>
            thumb.evaluate((image: HTMLImageElement) => ({
              complete: image.complete,
              width: image.naturalWidth,
              source: image.currentSrc,
            })),
          )
          .toMatchObject({ complete: true, width: 768 });
        const button = thumb.locator('..');
        await button.scrollIntoViewIfNeeded();
        await button.click();
        const dialog = page.getByRole('dialog');
        const full = dialog.locator('img');
        await expect(full).toHaveAttribute('data-asset-id', `npc.${archetype}.v2`);
        await expect
          .poll(() =>
            full.evaluate(
              (image: HTMLImageElement) => image.complete && image.naturalWidth === 768,
            ),
          )
          .toBe(true);
        const bounds = await full.boundingBox();
        expect(bounds?.height).toBeLessThanOrEqual(
          (width === 960 ? 640 : width === 1280 ? 800 : 1080) * 0.65 + 1,
        );
        await page.screenshot({
          path: `.artifacts/visual-production/npc-${archetype}-${lang}-${width}.png`,
          animations: 'disabled',
        });
        await page.keyboard.press('Escape');
        await expect(dialog).toHaveCount(0);
        await expect(button).toBeFocused();
      }
      await page.screenshot({
        path: `.artifacts/visual-production/npc-grid-${lang}-${width}.png`,
        animations: 'disabled',
      });
      const firstRecord = records[0];
      const changedRole = roles[lang][4];
      if (!firstRecord || !changedRole) throw Error('NPC fixture role missing');
      firstRecord.role = changedRole;
      await page.reload();
      await page.getByRole('button', { name: lang === 'en' ? 'NPCs' : 'NPC', exact: true }).click();
      await expect(page.locator('img[data-asset-id="npc.innkeeper.v2"]')).toHaveCount(1);
      await expect(page.locator('img[data-asset-id="npc.guard.legacy.v1"]')).toHaveCount(1);
    });
  }
