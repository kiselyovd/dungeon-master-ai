import { expect, test } from '@playwright/test';
import { emptyPcData, PRESETS } from '../src/state/pc';
import { mockTauri } from './fixtures/tauri-mock';

const ids = [
  'e9f54b09-a672-4ccf-97df-69000baf0b01',
  'e9f54b09-a672-4ccf-97df-69000baf0b02',
  'e9f54b09-a672-4ccf-97df-69000baf0b03',
];
const sessions = [
  '8954da8a-74cf-4e39-8350-d45e35b20101',
  '8954da8a-74cf-4e39-8350-d45e35b20102',
  '8954da8a-74cf-4e39-8350-d45e35b20103',
];

test('real production root switches whole campaigns and resumes the selected hero after reload', async ({
  page,
}) => {
  await mockTauri(page, {
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:31415/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
    active_campaign_id: ids[0],
    active_session_id: sessions[0],
  });
  const titles = ['First chronicle', 'Second chronicle', 'Empty chronicle'];
  const states = ids.map((id, index) => ({
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign: {
      id,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: titles[index],
      premise: '',
      seed: { kind: 'collaborative', answers: { genre: ['fantasy'] } },
      last_session_id: sessions[index],
      revision: 3,
      created_at: '2026-10-06T00:00:00Z',
      updated_at: '2026-10-06T00:00:00Z',
    },
    session_id: sessions[index],
    character_id: index === 2 ? null : id,
    projection: {
      messages: [],
      character:
        index === 2
          ? {}
          : {
              ...emptyPcData(),
              ...PRESETS.fighter,
              heroClass: 'fighter',
              characterId: id,
              name: index === 0 ? 'Mira' : 'Daren',
              inventory: [
                { id: 'personal', name: index === 0 ? 'Mira rope' : 'Daren book', count: 1 },
              ],
            },
      combat: null,
      scene:
        index === 2
          ? null
          : { id, title: index === 0 ? 'Mira quay' : 'Daren archive', mode: 'exploration' },
      journal: [],
      npcs: [],
    },
  }));
  await page.route(
    /^https?:\/\/[^/]+\/campaigns(?:\/[^/?]+(?:\/state)?)?(?:\?|$)/,
    async (route) => {
      const segments = new URL(route.request().url()).pathname.split('/');
      const state = states.find((item) => item.campaign.id === segments[2]);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(
          segments[2]
            ? segments[3] === 'state'
              ? state
              : state?.campaign
            : states.map((item) => item.campaign),
        ),
      });
    },
  );
  await page.route(/^https?:\/\/[^/]+\/sessions\//, async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' });
  });
  await page.goto('/');
  await expect(page.locator('.dm-char-fab')).toContainText('Mira');
  await expect(page.getByText('Mira quay', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Campaigns', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'First chronicle' })).toBeVisible();
  await page.getByRole('button', { name: /Second chronicle/ }).click();
  await expect(page.locator('.dm-char-fab')).toContainText('Daren');
  await expect(page.getByText('Daren archive', { exact: true })).toBeVisible();
  await expect(page.getByText('Mira quay', { exact: true })).toHaveCount(0);
  await page.locator('.dm-char-fab').click();
  const heroPanel = page.getByRole('tabpanel', { name: 'Hero', exact: true });
  await expect(heroPanel.getByRole('listitem').filter({ hasText: 'Daren book' })).toHaveText(
    'Daren book × 1',
  );
  await expect(heroPanel.getByRole('listitem').filter({ hasText: 'Mira rope' })).toHaveCount(0);
  await page.keyboard.press('Escape');
  await page.reload();
  await expect(page.locator('.dm-char-fab')).toContainText('Daren');
  await page.screenshot({ path: '.artifacts/campaign-resume.png', fullPage: true });
  await page.getByRole('button', { name: 'Campaigns', exact: true }).click();
  await page.getByRole('button', { name: /Empty chronicle/ }).click();
  await expect(
    page.getByRole('heading', { name: 'Hero for this adventure', exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'Create your own hero', exact: true }),
  ).toBeVisible();
  await expect(page.locator('.dm-char-fab')).toHaveCount(0);
});
