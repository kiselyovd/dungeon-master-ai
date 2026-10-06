import { randomUUID } from 'node:crypto';
import { type APIRequestContext, expect, type Page, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(
  !backend,
  'Requires an isolated real app-server; never substitute simulated progression.',
);

async function openAdventure(page: Page, request: APIRequestContext) {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'The Bell Beneath the Water',
      premise: 'A missing ferryman',
      seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
    },
  });
  expect(created.ok()).toBeTruthy();
  const campaign = await created.json();
  const hero = randomUUID();
  const committed = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: hero,
      expected_revision: 1,
      causation_id: randomUUID(),
      character: {
        name: 'Mira Veil',
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
  expect(committed.ok()).toBeTruthy();
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:9/v1',
        model: 'acceptance',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
    hero_class: 'fighter',
    active_campaign_id: campaign.id,
    active_session_id: campaign.last_session_id,
  });
  await page.route(
    /^https?:\/\/[^/]+\/(?:campaigns(?:\/|$)|sessions(?:\/|$)|saves(?:\/|$))/,
    async (route) => {
      const url = new URL(route.request().url());
      const response = await route.fetch({ url: `${base}${url.pathname}${url.search}` });
      await route.fulfill({ response });
    },
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'The Bell Beneath the Water' });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(/land ledger proves/)).toHaveCount(0);
  const click = async (name: string) => {
    const button = dialog.getByRole('button', { name, exact: true });
    await expect(button).toBeEnabled();
    const response = page.waitForResponse(
      (response) =>
        response.url().endsWith('/adventure/actions') && response.request().method() === 'POST',
    );
    await button.click();
    expect((await response).status()).toBe(200);
    await expect(dialog.getByText('Confirming your action…')).toHaveCount(0);
    await expect(dialog.getByRole('alert')).toHaveCount(0);
    await expect(dialog.locator('ol li').last()).toHaveText(name);
  };
  return { base, campaign, hero, dialog, click };
}

test('production UI completes the real authored peaceful route and reloads its ending', async ({
  page,
  request,
}) => {
  const { base, campaign, dialog, click } = await openAdventure(page, request);
  for (const name of [
    'Go to Old quay',
    'Inspect the ward and river tracks',
    'Go to Flooded archive',
    'Call for Tikhon in the upper chamber',
    'Escort Tikhon home along the safe route',
    'Go to Old quay',
    'Go to Veresk',
    'Go to Town shrine',
    'Ask Leon about the ward and roster',
    'Go to Veresk',
    "Go to Varden's warehouse",
    'Negotiate the return of the clapper',
    'Straighten and repair the clapper',
    'Go to Veresk',
    'Go to Old quay',
    'Go to Flooded archive',
    'Go to Lower belfry',
    "Acknowledge Savely's last rescue",
    'Repair the inscription and sound the memorial bell',
    'Go to Flooded archive',
    'Go to Old quay',
    'Go to Veresk',
    'Record the outcome of this session',
  ])
    await click(name);
  await expect(dialog.getByRole('status')).toHaveText(/Remembered river/);
  await page.screenshot({ path: '.artifacts/bell-adventure/peaceful-ending.png', fullPage: true });
  await page.reload();
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Remembered river' })).toBeVisible();
  const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(current.projection.adventure.state.ending).toBe('remembered_river');
  expect(
    current.projection.character.inventory.filter(
      (item: { id: string }) => item.id === 'bell-rescue-gold',
    ),
  ).toEqual([expect.objectContaining({ count: 10 })]);
});

test('sealed and unresolved outcomes remain distinct in the real production UI', async ({
  page,
  request,
}) => {
  const { base, campaign, dialog, click } = await openAdventure(page, request);
  await click('Record the outcome of this session');
  await expect(dialog.getByRole('status')).toHaveText(/Unfinished debt/);
  for (const name of [
    'Go to Old quay',
    'Go to Flooded archive',
    'Call for Tikhon in the upper chamber',
    'Escort Tikhon home along the safe route',
    'Take the optional land ledger',
    'Go to Lower belfry',
    'Seal the lower sluice and close the crossing',
    'Go to Flooded archive',
    'Go to Old quay',
    'Go to Veresk',
    'Present the recovered ledger to the town council',
    'Record the outcome of this session',
  ])
    await click(name);
  await expect(dialog.getByRole('status')).toHaveText(/Closed crossing/);
  const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(current.projection.adventure.state.facts).toContain('ledger_published');
  expect(current.projection.adventure.state.facts).not.toContain('ward_restored');
  await page.screenshot({ path: '.artifacts/bell-adventure/sealed-ending.png', fullPage: true });
});

test('real snapshot restore rewinds adventure facts and reward, then permits a fresh action', async ({
  page,
  request,
}) => {
  const { base, campaign, dialog, click } = await openAdventure(page, request);
  await click('Go to Old quay');
  await click('Go to Flooded archive');
  await click('Call for Tikhon in the upper chamber');
  const before = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  const saved = await request.post(`${base}/sessions/${campaign.last_session_id}/saves`, {
    data: {
      campaign_id: campaign.id,
      expected_revision: before.campaign.revision,
      causation_id: randomUUID(),
      character: {},
      kind: 'manual',
      title: 'Before rescue',
      summary: 'Tikhon located',
      tag: 'exploration',
    },
  });
  expect(saved.ok()).toBeTruthy();
  const checkpoint = await saved.json();
  await page.reload();
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await click('Escort Tikhon home along the safe route');
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.getByRole('button', { name: 'Saves', exact: true }).click();
  await page.getByRole('button', { name: /Before rescue/ }).click();
  const restoreResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/saves/${checkpoint.id}/restore`) &&
      response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: 'Load', exact: true }).click();
  expect((await restoreResponse).status()).toBe(200);
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(
    dialog.getByRole('button', { name: 'Escort Tikhon home along the safe route' }),
  ).toBeEnabled();
  await click('Escort Tikhon home along the safe route');
  const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(
    current.projection.character.inventory.filter(
      (item: { id: string }) => item.id === 'bell-rescue-gold',
    ),
  ).toEqual([expect.objectContaining({ count: 10 })]);
});
