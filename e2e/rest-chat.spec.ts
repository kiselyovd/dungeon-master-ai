import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

test('a terminal DM combat result closes the real tabletop and updates the adventure', async ({
  page,
}) => {
  const campaign = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
    session = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb',
    character = '92414394-f4b1-477f-9278-10f75c3c4fb7',
    encounter = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';
  const adventure = (active: boolean) => ({
    state: {
      package_id: 'bell-beneath-water',
      location: active ? 'archive' : 'quay',
      facts: ['ferry_route', 'tikhon_located'],
      ending: null,
      closed: false,
      history: [],
      defense: active ? 'active' : 'retreated',
      defense_encounter_id: encounter,
    },
    available_actions: [],
  });
  const projection = (active: boolean) => ({
    schema_version: 1,
    encounter_id: encounter,
    revision: active ? 1 : 2,
    campaign_scope: {
      campaign_id: campaign,
      session_id: session,
      character_id: character,
      revision: active ? 2 : 3,
    },
    adventure: adventure(active),
    snapshot: {
      active,
      round: 1,
      current_combatant: active ? 'hero-token' : null,
      initiative: [{ id: 'hero-token', roll: 18, dex_tiebreak: 2 }],
      combatants: [
        {
          id: 'hero-token',
          character_id: character,
          name: 'Mira',
          max_hp: 12,
          current_hp: 4,
          temp_hp: 0,
          ac: 14,
          speed_ft: 30,
          initiative_roll: 18,
          dex_mod: 2,
          conditions: [],
          budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
          is_dead: false,
          position: { x: 1, y: 3 },
        },
      ],
    },
    events: [],
  });
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
    active_campaign_id: campaign,
    active_session_id: session,
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:9/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
  });
  await page.route('**/campaigns/*/state', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        campaign: {
          id: campaign,
          last_session_id: session,
          revision: 2,
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          title: 'Bell',
          premise: 'Ferry',
          seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
          created_at: '2026-10-07T00:00:00Z',
          updated_at: '2026-10-07T00:00:00Z',
        },
        session_id: session,
        character_id: character,
        projection: {
          character: {
            characterId: character,
            name: 'Mira',
            heroClass: 'fighter',
            level: 1,
            hp: 4,
            hpMax: 12,
            ac: 14,
            inventory: [],
          },
          messages: [],
          combat: projection(true),
          adventure: adventure(true),
          scene: null,
          journal: [],
          npcs: [],
        },
      }),
    }),
  );
  await page.route('**/agent/turn', (route) => {
    const frames = [
      ['tool_call_start', { id: 'retreat-1', tool_name: 'combat_action', round: 1 }],
      [
        'tool_call_result',
        {
          id: 'retreat-1',
          tool_name: 'combat_action',
          args: { kind: 'retreat' },
          result: { projection: projection(false) },
          is_error: false,
          handled_by: 'engine',
          round: 1,
        },
      ],
      ['text_delta', { text: 'You withdraw to the quay. Tikhon still needs rescue.' }],
      ['agent_done', { total_rounds: 2 }],
    ];
    return route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: frames
        .map(([event, data]) => `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
        .join(''),
    });
  });
  await page.goto('/');
  await expect(page.locator('.dm-actionbar')).toBeVisible();
  await page
    .getByRole('textbox', { name: 'What do you do?' })
    .fill('I retreat through the dry exit.');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(
    page.getByText('You withdraw to the quay. Tikhon still needs rescue.', { exact: true }),
  ).toBeVisible();
  await expect(page.locator('.dm-actionbar')).toHaveCount(0);
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(
    page.getByRole('dialog').getByRole('heading', { name: 'Old quay', exact: true }),
  ).toBeVisible();
});

test('a confirmed DM rest updates the real root HP without interrupting narration', async ({
  page,
}) => {
  const campaign = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const session = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
  const character = '92414394-f4b1-477f-9278-10f75c3c4fb7';
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
    pc: { name: 'Mira', hp: 4, hpMax: 12 },
    active_campaign_id: campaign,
    active_session_id: session,
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:9/v1',
        model: 'fixture-dm',
        apiKey: 'fixture-key',
      },
      'local-mistralrs': null,
    },
    image_enabled: false,
    video_enabled: false,
  });
  await page.route('**/agent/turn', (route) => {
    const result = {
      rest_status: {
        campaign_id: campaign,
        session_id: session,
        character_id: character,
        revision: 3,
        hp: 7,
        max_hp: 12,
        hit_die: 10,
        hit_dice_remaining: 0,
        slots: {},
        elapsed_seconds: 3600,
        long_rest_ready_in_seconds: 0,
        safe: true,
        combat_active: false,
      },
    };
    const frames = [
      ['tool_call_start', { id: 'rest-1', tool_name: 'rest_action', round: 1 }],
      [
        'tool_call_result',
        {
          id: 'rest-1',
          tool_name: 'rest_action',
          args: { kind: 'short', hit_dice: 1 },
          result,
          is_error: false,
          handled_by: 'engine',
          round: 1,
        },
      ],
      ['text_delta', { text: 'You bind the wound and rest at the inn.' }],
      ['agent_done', { total_rounds: 2 }],
    ];
    return route.fulfill({
      status: 200,
      contentType: 'text/event-stream',
      body: frames
        .map(([event, data]) => `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`)
        .join(''),
    });
  });
  await page.goto('/');
  await expect(page.getByRole('button', { name: 'Open character sheet for Mira' })).toContainText(
    '4/12',
  );
  await page
    .getByRole('textbox', { name: 'What do you do?' })
    .fill('I take a short rest and spend one hit die.');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(
    page.getByText('You bind the wound and rest at the inn.', { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open character sheet for Mira' })).toContainText(
    '7/12',
  );
  await expect(
    page.getByText('I take a short rest and spend one hit die.', { exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: '.artifacts/dm-hero/rest-chat.png', fullPage: true });
});
