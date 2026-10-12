import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { expect, test } from '@playwright/test';
import { emptyPcData, PRESETS } from '../src/state/pc';
import { mockTauri } from './fixtures/tauri-mock';

test('production tabletop resolves the same persisted map after reload without inline bytes', async ({
  page,
}) => {
  const campaignId = 'deb3a4e3-5ed9-4f59-9a8b-890c4f87291d';
  const sessionId = '62ba8579-904b-4460-a68a-df6268ed404a';
  const actorId = 'b5271021-b82c-40c4-aa9f-554af0d79b22';
  const assetId = 'a2843520-2a22-4d82-b34d-33ae04389f41';
  const bytes = await readFile('src/assets/living-tabletop/v2/map-archive-v2.webp');
  const mapRef = {
    source: 'owned',
    asset_id: assetId,
    category: 'map',
    origin: 'generated',
    descriptor: {
      sha256: createHash('sha256').update(bytes).digest('hex'),
      mime_type: 'image/webp',
      byte_len: bytes.length,
      width: 1536,
      height: 1024,
    },
  };
  const campaign = {
    id: campaignId,
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    title: 'Map reference fixture',
    premise: 'Map lookup',
    seed: { kind: 'collaborative', answers: { genre: ['classic_fantasy'] } },
    last_session_id: sessionId,
    revision: 3,
    created_at: '2026-10-12T00:00:00Z',
    updated_at: '2026-10-12T00:00:00Z',
  };
  const state = {
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign,
    session_id: sessionId,
    character_id: actorId,
    projection: {
      character: {
        ...emptyPcData(),
        ...PRESETS.fighter,
        heroClass: 'fighter',
        name: 'Map hero',
        characterId: actorId,
      },
      messages: [],
      combat: null,
      scene: null,
      adventure: null,
      journal: [],
      npcs: [],
      map_media_ref: mapRef,
    },
  };
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_campaign_id: campaignId,
    active_session_id: sessionId,
    ui_language: 'en',
    image_enabled: false,
  });
  await page.route('**/campaigns/*/state', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(state) }),
  );
  await page.route(`**/media/assets/${assetId}?*`, (route) => {
    expect(new URL(route.request().url()).searchParams.get('campaign_id')).toBe(campaignId);
    return route.fulfill({ status: 200, contentType: 'image/webp', body: bytes });
  });
  await page.goto('/');
  const image = page.getByTestId('dm-vtt-map-bg');
  await expect(image).toHaveAttribute('src', /^blob:/);
  await expect
    .poll(() =>
      image.evaluate(
        (node) => node instanceof HTMLImageElement && node.complete && node.naturalWidth === 1536,
      ),
    )
    .toBe(true);
  await page.reload();
  await expect(image).toHaveAttribute('src', /^blob:/);
  await expect
    .poll(() =>
      image.evaluate(
        (node) => node instanceof HTMLImageElement && node.complete && node.naturalHeight === 1024,
      ),
    )
    .toBe(true);
  const settings = await page.evaluate(() => localStorage.getItem('dmai-e2e-store:settings.json'));
  expect(settings).not.toContain('data:image/');
  expect(settings).not.toContain('blob:');
  await page.screenshot({ path: '.artifacts/product-uiux/map-reference-production.png' });
});
