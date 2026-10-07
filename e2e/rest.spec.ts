import { randomUUID } from 'node:crypto';
import fs from 'node:fs';
import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const backend = process.env.DMAI_ADVENTURE_BACKEND_URL;
test.skip(!backend, 'Requires isolated real backend.');
test('rest recovery after reload reuses its request, then long rest preserves ammunition and cooldown', async ({
  page,
  request,
}) => {
  const base = backend as string;
  const created = await request.post(`${base}/campaigns`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Rest acceptance',
      premise: 'Town',
      seed: { kind: 'collaborative', answers: { world: ['Town'] } },
    },
  });
  expect(created.ok()).toBeTruthy();
  const campaign = await created.json();
  const characterId = randomUUID();
  const hero = await request.put(`${base}/campaigns/${campaign.id}/character`, {
    data: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: campaign.last_session_id,
      character_id: characterId,
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
        inventory: [{ id: 'arrows-20', name: 'Arrows', count: 1 }],
        spells: { cantrips: ['sacred-flame'], level1: ['cure-wounds'] },
        combatResources: { arrows: 19, hitDiceRemaining: 1, spellSlots: { 1: 0 } },
      },
    },
  });
  expect(hero.ok()).toBeTruthy();
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
  let dropped = false;
  const restRequests: Record<string, unknown>[] = [];
  let shortOutcome: unknown;
  let longOutcome: unknown;
  await page.route(
    /^https?:\/\/[^/]+\/(?:campaigns(?:\/|$)|sessions(?:\/|$)|saves(?:\/|$)|combat(?:\/|$))/,
    async (route) => {
      const url = new URL(route.request().url());
      const response = await route.fetch({ url: `${base}${url.pathname}${url.search}` });
      if (url.pathname.endsWith('/rest') && route.request().method() === 'POST') {
        const body = route.request().postDataJSON();
        restRequests.push(body);
        if (body.intent.kind === 'short' && !shortOutcome) shortOutcome = await response.json();
        if (body.intent.kind === 'long') longOutcome = await response.json();
        if (!dropped) {
          dropped = true;
          await route.abort('failed');
          return;
        }
      }
      await route.fulfill({ response });
    },
  );
  await page.goto('/');
  await page.locator('.dm-char-fab').click();
  await page.getByRole('button', { name: 'Rest', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Rest', exact: true });
  await expect(dialog).toContainText('HP: 2 / 20');
  await dialog.getByRole('combobox', { name: 'Hit dice to spend' }).click();
  await dialog.getByRole('option', { name: '1', exact: true }).click();
  await dialog.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Retry the same rest' })).toBeVisible();
  await page.reload();
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Retry the same rest' }).click();
  await expect(dialog).toContainText('Hit dice: 0d8 remaining');
  expect(restRequests[1]?.causation_id).toBe(restRequests[0]?.causation_id);
  expect(restRequests[1]).toEqual(restRequests[0]);
  const afterShort = await (await request.get(`${base}/campaigns/${campaign.id}/rest`)).json();
  expect(afterShort.elapsed_seconds).toBe(3600);
  expect(afterShort.hp).toBeGreaterThan(2);
  await dialog.getByRole('combobox', { name: 'Activity' }).click();
  await dialog.getByRole('option', { name: 'Long rest · 8 hours', exact: true }).click();
  await dialog.getByRole('button', { name: 'Confirm', exact: true }).click();
  await expect(dialog).toContainText('HP: 20 / 20');
  await expect(dialog).toContainText('Level 1 slots: 2 / 2');
  await expect(dialog).toContainText('Adventure time: 9h 00m');
  await expect(dialog.getByRole('button', { name: 'Confirm', exact: true })).toBeDisabled();
  const state = await (await request.get(`${base}/campaigns/${campaign.id}/state`)).json();
  expect(state.projection.character.combatResources.arrows).toBe(19);
  expect(state.projection.adventuring.elapsed_seconds).toBe(32400);
  await page.screenshot({
    path: '.artifacts/rest-browser/completed.png',
    fullPage: true,
    animations: 'disabled',
  });
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await page.reload();
  await page.locator('.dm-char-fab').click();
  await page.getByRole('button', { name: 'Rest', exact: true }).click();
  await expect(dialog).toContainText('Adventure time: 9h 00m');
  await expect(dialog).toContainText('Another long rest can start in 16 hours.');
  fs.mkdirSync('.artifacts/rest-browser', { recursive: true });
  fs.writeFileSync(
    '.artifacts/rest-browser/restart-probe.json',
    JSON.stringify(
      {
        campaignId: campaign.id,
        shortRequest: restRequests[0],
        longRequest: restRequests[2],
        shortOutcome,
        longOutcome,
        expectedHp: 20,
        expectedElapsedSeconds: 32400,
      },
      null,
      2,
    ),
  );
});
