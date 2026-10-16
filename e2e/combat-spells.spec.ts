import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires the isolated real backend.');
test('production spell selection heals durably, spends one slot and retains a cantrip action', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Spell acceptance',
      premise: 'A bridge',
      seed: { kind: 'collaborative', answers: { world: ['Bridge'] } },
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
        name: 'Mira',
        heroClass: 'cleric',
        level: 1,
        hp: 2,
        hpMax: 20,
        ac: 16,
        abilities: { str: 12, dex: 14, con: 14, int: 10, wis: 16, cha: 8 },
        inventory: [],
        spells: { cantrips: ['sacred-flame'], level1: ['healing-word', 'cure-wounds', 'bless'] },
      },
    },
  });
  expect(committed.ok()).toBeTruthy();
  const started = await request.post(`${base}/combat/start`, {
    data: {
      campaign_id: campaign.id,
      session_id: campaign.last_session_id,
      initiative_entries: [
        { id: randomUUID(), name: 'Foe', roll: 0, dex_mod: 0, hp: 100, max_hp: 100, ac: 10 },
      ],
    },
  });
  expect(started.ok()).toBeTruthy();
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'cleric',
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
  await page.getByRole('button', { name: 'Cast C', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Cast a spell' });
  await dialog.getByRole('combobox', { name: 'Prepared spell' }).click();
  await dialog.getByRole('option', { name: 'Healing Word', exact: true }).click();
  await expect(dialog.getByRole('combobox', { name: 'Spell slot' })).toContainText('2 / 2');
  await page.screenshot({
    path: '.artifacts/combat-spells/healing-word.png',
    fullPage: true,
    animations: 'disabled',
  });
  const reply = page.waitForResponse(
    (response) =>
      response.url().endsWith('/combat/action') && response.request().method() === 'POST',
  );
  await dialog.getByRole('button', { name: 'Confirm spell' }).click();
  const response = await reply;
  expect(response.status()).toBe(200);
  const body = response.request().postDataJSON();
  expect(body.action_type).toBe('spell');
  expect(body.args.spell_id).toBe('healing-word');
  expect(body.args.slot_level).toBe(1);
  expect(body.args.damage_dice).toBeUndefined();
  await expect(dialog).toHaveCount(0);
  const current = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(current.projection.character.hp).toBeGreaterThan(2);
  expect(current.projection.character.combatResources.spellSlots['1']).toBe(1);
  const hero = current.projection.combat.snapshot.combatants.find(
    (actor: { character_id?: string }) => actor.character_id,
  );
  expect(hero.budget.action).toBe(true);
  expect(hero.budget.bonus_action).toBe(false);
  await page.reload();
  await page.getByRole('button', { name: 'Cast C', exact: true }).click();
  await expect(dialog.getByRole('combobox', { name: 'Prepared spell' })).toContainText(
    'Sacred Flame',
  );
  const cantrip = page.waitForResponse(
    (response) =>
      response.url().endsWith('/combat/action') && response.request().method() === 'POST',
  );
  await dialog.getByRole('button', { name: 'Confirm spell' }).click();
  expect((await cantrip).status()).toBe(200);
  const final = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(final.projection.character.combatResources.spellSlots['1']).toBe(1);
  await expect(page.getByRole('button', { name: 'Cast C', exact: true })).toBeDisabled();
});
