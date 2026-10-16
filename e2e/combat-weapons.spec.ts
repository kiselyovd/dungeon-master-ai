import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires an isolated real backend for canonical combat.');
test('production weapon control completes real combat and reloads its durable result', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Weapon combat acceptance',
      premise: 'A practice encounter',
      seed: { kind: 'collaborative', answers: { world: ['Bridge'] } },
    },
  });
  expect(created.ok()).toBeTruthy();
  const campaign = await created.json();
  const character = randomUUID();
  const committed = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: character,
      expected_revision: 1,
      causation_id: randomUUID(),
      character: {
        name: 'Daren',
        heroClass: 'fighter',
        level: 1,
        hp: 12,
        hpMax: 12,
        ac: 16,
        abilities: { str: 16, dex: 14, con: 14, int: 10, wis: 12, cha: 8 },
        inventory: [{ id: 'longsword', name: 'Longsword', count: 1 }],
      },
    },
  });
  expect(committed.ok()).toBeTruthy();
  const started = await request.post(`${base}/combat/start`, {
    data: {
      campaign_id: campaign.id,
      session_id: campaign.last_session_id,
      initiative_entries: [
        { id: randomUUID(), name: 'Practice foe', roll: 0, dex_mod: 0, hp: 2, max_hp: 2, ac: 1 },
      ],
    },
  });
  expect(started.ok()).toBeTruthy();
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
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
  await expect(page.locator('.dm-actionbar')).toBeVisible();
  let ended = false;
  for (let attempt = 0; attempt < 20 && !ended; attempt++) {
    await page.getByRole('button', { name: 'Attack A', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'Weapon attack' });
    await expect(dialog).toBeVisible();
    if (attempt === 0)
      await page.screenshot({
        path: '.artifacts/combat-weapons/weapon-dialog.png',
        fullPage: true,
        animations: 'disabled',
      });
    const response = page.waitForResponse(
      (response) =>
        response.url().endsWith('/combat/action') && response.request().method() === 'POST',
    );
    await dialog.getByRole('button', { name: 'Confirm attack' }).click();
    const confirmed = await response;
    expect(confirmed.status()).toBe(200);
    const body = confirmed.request().postDataJSON();
    expect(body.action_type).toBe('weapon_attack');
    expect(body.args.weapon_id).toBe('longsword');
    expect(body.args.attack_modifier).toBeUndefined();
    expect(body.args.damage_dice).toBeUndefined();
    await expect(dialog).toHaveCount(0);
    const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
    ended = current.projection.combat === null;
    if (!ended) {
      const advance = page.waitForResponse(
        (response) =>
          response.url().endsWith('/combat/action') && response.request().method() === 'POST',
      );
      await page.getByRole('button', { name: /End turn/i }).click();
      expect((await advance).status()).toBe(200);
    }
  }
  expect(ended).toBe(true);
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  await page.reload();
  await expect(page.locator('.dm-char-fab')).toContainText('Daren');
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(current.character_id).toBe(character);
  expect(current.projection.combat).toBeNull();
  await page.screenshot({ path: '.artifacts/combat-weapons/completed.png', fullPage: true });
});
