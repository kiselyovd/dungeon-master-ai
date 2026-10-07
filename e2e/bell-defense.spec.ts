import { randomUUID } from 'node:crypto';
import { expect, type Page, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires isolated real backend.');
async function act(page: Page, label: string) {
  const reply = page.waitForResponse(
    (response) =>
      response.url().includes('/adventure/actions') && response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: label, exact: true }).click({ timeout: 15000 });
  expect((await reply).status()).toBe(200);
}
test('a complete authored defensive route persists its map and reaches a rescued sealed ending', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Defensive Bell session',
      premise: 'A missing ferryman',
      seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
    },
  });
  expect(created.ok()).toBeTruthy();
  const campaign = await created.json();
  const committed = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: randomUUID(),
      expected_revision: 1,
      causation_id: randomUUID(),
      character: {
        name: 'Nora',
        heroClass: 'wizard',
        level: 1,
        hp: 8,
        hpMax: 8,
        ac: 12,
        abilities: { str: 8, dex: 14, con: 14, int: 16, wis: 12, cha: 10 },
        inventory: [{ id: 'quarterstaff', name: 'Quarterstaff', count: 1 }],
        spells: { cantrips: ['fire-bolt'], level1: ['magic-missile'] },
      },
    },
  });
  expect(committed.ok()).toBeTruthy();
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'wizard',
    active_campaign_id: campaign.id,
    active_session_id: campaign.last_session_id,
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
  });
  await page.route(
    /^https?:\/\/[^/]+\/(?:campaigns(?:\/|$)|sessions(?:\/|$)|saves(?:\/|$)|combat(?:\/|$))/,
    async (route) => {
      const url = new URL(route.request().url());
      const response = await route.fetch({ url: `${base}${url.pathname}${url.search}` });
      await route.fulfill({ response });
    },
  );
  await page.goto('/');
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await act(page, 'Go to Old quay');
  await act(page, 'Go to Flooded archive');
  await act(page, 'Call for Tikhon in the upper chamber');
  await act(page, 'Hold the passage for Tikhon');
  await page
    .getByRole('dialog', { name: 'The Bell Beneath the Water' })
    .getByRole('button', { name: 'Close', exact: true })
    .click();
  await expect(page.getByTestId('authored-map')).toHaveAttribute('data-width-cells', '12');
  await expect(page.getByTestId('authored-wall')).toHaveCount(7);
  await expect(page.getByRole('button', { name: 'Retreat X', exact: true })).toBeDisabled();
  await page.reload();
  await expect(page.getByTestId('authored-map')).toBeVisible();
  await expect(page.locator('.dm-init-list')).toContainText('Water echo 1');
  await page.screenshot({
    path: '.artifacts/bell-defense/active-map.png',
    fullPage: true,
    animations: 'disabled',
  });
  // The partition blocks the first shot. Validation must preserve both slots.
  await page.getByRole('button', { name: 'Cast C', exact: true }).click();
  const blockedDialog = page.getByRole('dialog', { name: 'Cast a spell' });
  await blockedDialog.getByRole('combobox', { name: 'Prepared spell' }).click();
  await blockedDialog.getByRole('option', { name: 'Magic Missile', exact: true }).click();
  const blocked = page.waitForResponse(
    (response) =>
      response.url().endsWith('/combat/action') && response.request().method() === 'POST',
  );
  await blockedDialog.getByRole('button', { name: 'Confirm spell' }).click();
  expect((await blocked).status()).toBe(400);
  await blockedDialog.getByRole('button', { name: 'Cancel', exact: true }).click();
  const before = (await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json())
    .projection.character;
  expect(before.combatResources.spellSlots['1']).toBe(2);
  const openingTurn = page.waitForResponse(
    (response) =>
      response.url().endsWith('/combat/action') && response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: /End turn/i }).click();
  expect((await openingTurn).status()).toBe(200);
  for (let index = 0; index < 2; index++) {
    await page.getByRole('button', { name: 'Cast C', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Cast a spell' });
    await dialog.getByRole('combobox', { name: 'Prepared spell' }).click();
    await dialog.getByRole('option', { name: 'Magic Missile', exact: true }).click();
    const reply = page.waitForResponse(
      (response) =>
        response.url().endsWith('/combat/action') && response.request().method() === 'POST',
    );
    await dialog.getByRole('button', { name: 'Confirm spell' }).click();
    expect((await reply).status()).toBe(200);
    if (index === 0) {
      const turn = page.waitForResponse(
        (response) =>
          response.url().endsWith('/combat/action') && response.request().method() === 'POST',
      );
      await page.getByRole('button', { name: /End turn/i }).click();
      expect((await turn).status()).toBe(200);
    }
  }
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'The Bell Beneath the Water' })).toContainText(
    'The passage is held.',
  );
  await act(page, 'Escort Tikhon home along the safe route');
  await act(page, 'Go to Lower belfry');
  await act(page, 'Seal the lower sluice and close the crossing');
  await act(page, 'Go to Flooded archive');
  await act(page, 'Go to Old quay');
  await act(page, 'Go to Veresk');
  await act(page, 'Record the outcome of this session');
  await expect(page.getByRole('dialog', { name: 'The Bell Beneath the Water' })).toContainText(
    'Closed crossing:',
  );
  await page.reload();
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'The Bell Beneath the Water' })).toContainText(
    'Closed crossing:',
  );
  const state = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(state.projection.adventure.state.defense).toBe('held');
  expect(state.projection.adventure.state.facts).not.toContain('ward_restored');
  expect(state.projection.character.combatResources.spellSlots['1']).toBe(0);
  expect(
    state.projection.character.inventory.filter(
      (item: { id: string }) => item.id === 'bell-rescue-gold',
    ),
  ).toHaveLength(1);
  await page.screenshot({
    path: '.artifacts/bell-defense/completed-session.png',
    fullPage: true,
    animations: 'disabled',
  });
});
