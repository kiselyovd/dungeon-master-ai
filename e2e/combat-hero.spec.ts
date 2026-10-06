import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires an isolated real backend and durable character.');
test('real campaign continuation preserves the bound hero among same-name NPCs', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Hero binding acceptance',
      premise: 'A bridge encounter',
      seed: { kind: 'collaborative', answers: { world: ['Bridge'] } },
    },
  });
  expect(created.ok()).toBeTruthy();
  const campaign = await created.json();
  const character = randomUUID();
  const npc = randomUUID();
  const committed = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: character,
      expected_revision: 1,
      causation_id: randomUUID(),
      character: {
        name: 'Mira',
        heroClass: 'fighter',
        level: 1,
        hp: 7,
        hpMax: 12,
        ac: 16,
        speedFt: 35,
        abilities: { str: 16, dex: 14, con: 14, int: 10, wis: 12, cha: 8 },
        inventory: [],
      },
    },
  });
  expect(committed.ok()).toBeTruthy();
  const started = await request.post(`${base}/combat/start`, {
    data: {
      campaign_id: campaign.id,
      session_id: campaign.last_session_id,
      initiative_entries: [
        { id: npc, name: 'Mira', roll: 30, dex_mod: 0, hp: 8, max_hp: 8, ac: 12 },
      ],
    },
  });
  expect(started.ok()).toBeTruthy();
  let current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  const projection = current.projection.combat;
  expect(projection.snapshot.combatants).toHaveLength(2);
  const hero = projection.snapshot.combatants.find(
    (actor: { character_id?: string }) => actor.character_id === character,
  );
  expect(hero).toBeTruthy();
  expect(hero.id).not.toBe(character);
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
  await expect(page.locator('.dm-char-fab')).toContainText('Mira');
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  const advance = await request.post(`${base}/combat/action`, {
    data: {
      encounter_id: projection.encounter_id,
      action_type: 'end_turn',
      request_id: randomUUID(),
      expected_revision: 0,
      args: { combatant_id: npc },
    },
  });
  expect(advance.ok()).toBeTruthy();
  await page.reload();
  await expect(page.locator('.dm-actionbar')).toBeVisible();
  await expect(page.locator('.dm-char-fab')).toContainText('7/12');
  const response = page.waitForResponse(
    (response) =>
      response.url().endsWith('/combat/action') && response.request().method() === 'POST',
  );
  await page.getByRole('button', { name: /End turn/i }).click();
  expect((await response).status()).toBe(200);
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(current.projection.character.hp).toBe(7);
  expect(current.projection.combat.snapshot.current_combatant).toBe(npc);
  await page.reload();
  await expect(page.locator('.dm-char-fab')).toContainText('7/12');
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  await page.screenshot({ path: '.artifacts/hero-binding/browser-continue.png', fullPage: true });
});
