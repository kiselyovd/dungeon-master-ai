import { expect, type Page, type Route, test } from '@playwright/test';
import type { CampaignStateResponse } from '../src/api/contracts/campaignState';
import { mockTauri } from './fixtures/tauri-mock';

const CAMPAIGN_ID = '00000000-0000-4000-8000-000000000001';
const SESSION_ID = '00000000-0000-4000-8000-000000000002';
const HERO_ID = '00000000-0000-4000-8000-000000001001';
const GOBLIN_ID = '00000000-0000-4000-8000-000000001002';
const ENCOUNTER_ID = '00000000-0000-4000-8000-000000001003';
const SAVE_ID = '00000000-0000-4000-8000-000000001004';
const SNAPSHOT_ID = '00000000-0000-4000-8000-000000001005';
let characterId: string | null = null;
const PIXEL_B64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAusB9Y9Zl1sAAAAASUVORK5CYII=';

type ProjectionOptions = {
  active?: boolean;
  current?: 'hero' | 'goblin' | null;
  heroX?: number;
  heroHp?: number;
  heroAction?: boolean;
  goblinHp?: number;
};

function projection(revision: number, options: ProjectionOptions = {}) {
  const {
    active = true,
    current = 'hero',
    heroX = 0,
    heroHp = 8,
    heroAction = true,
    goblinHp = 7,
  } = options;
  return {
    schema_version: 1,
    encounter_id: ENCOUNTER_ID,
    campaign_scope: {
      campaign_id: CAMPAIGN_ID,
      session_id: SESSION_ID,
      character_id: characterId,
      revision: revision + 3,
    },
    revision,
    snapshot: {
      active,
      round: revision >= 5 ? 3 : revision >= 3 ? 2 : 1,
      current_combatant: current === 'hero' ? HERO_ID : current === 'goblin' ? GOBLIN_ID : null,
      initiative: [
        { id: HERO_ID, roll: 18, dex_tiebreak: 2 },
        { id: GOBLIN_ID, roll: 12, dex_tiebreak: 1 },
      ],
      combatants: [
        {
          id: HERO_ID,
          character_id: characterId,
          loadout: {
            default_weapon: 'mace',
            attacks: [
              {
                weapon_id: 'mace',
                attack_bonus: 6,
                damage: { count: 1, die: 'D6', modifier: 4 },
                damage_type: 'bludgeoning',
                reach_ft: 5,
                normal_range_ft: 0,
                long_range_ft: 0,
                ranged: false,
                thrown: false,
                ammunition: false,
                two_handed: false,
                versatile_damage: null,
                heavy_disadvantage: false,
              },
            ],
            quantities: { mace: 1 },
            arrows: 0,
            shield: false,
          },
          casting: {
            level: 1,
            ability_modifier: 1,
            attack_bonus: 3,
            save_dc: 11,
            prepared: ['cure-wounds'],
            choices: [
              {
                id: 'cure-wounds',
                level: 1,
                range_ft: 5,
                bonus_action: false,
                healing: true,
                supported: true,
              },
            ],
            slots: { '1': { maximum: 2, remaining: revision >= 2 ? 1 : 2 } },
          },
          name: 'Aria',
          max_hp: 10,
          current_hp: heroHp,
          temp_hp: 0,
          ac: 12,
          speed_ft: 30,
          initiative_roll: 18,
          dex_mod: 2,
          conditions: [],
          budget: {
            action: heroAction,
            bonus_action: true,
            reaction: true,
            movement_ft: 30 - heroX * 5,
          },
          is_dead: false,
          position: { x: heroX, y: 0 },
        },
        {
          id: GOBLIN_ID,
          name: 'Goblin Scout',
          max_hp: 7,
          current_hp: goblinHp,
          temp_hp: 0,
          ac: 13,
          speed_ft: 30,
          initiative_roll: 12,
          dex_mod: 1,
          conditions: [],
          budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
          is_dead: goblinHp === 0,
          position: { x: 3, y: 0 },
        },
      ],
    },
    events: [],
  };
}

function event(name: string, data: unknown): string {
  return `event: ${name}\ndata: ${JSON.stringify(data)}\n\n`;
}

function toolStart(id: string, toolName: string): string {
  return event('tool_call_start', { id, tool_name: toolName, round: 1 });
}

function toolResult(id: string, toolName: string, args: unknown, result: unknown): string {
  return event('tool_call_result', {
    id,
    tool_name: toolName,
    args,
    result,
    is_error: false,
    round: 1,
    handled_by: 'engine',
  });
}

async function fulfillSse(route: Route, body: string): Promise<void> {
  await route.fulfill({
    status: 200,
    headers: { 'content-type': 'text/event-stream' },
    body,
  });
}

async function sendChat(page: Page, text: string): Promise<void> {
  const composer = page.locator('textarea').last();
  await composer.fill(text);
  await page.getByRole('button', { name: /^(Send|Отправить)$/ }).click();
}

const compendium = {
  races: [
    {
      id: 'human',
      ruleset_id: 'srd-5.2.1',
      name_key: 'catalog.species.human',
      name_en: 'Human',
      name_ru: 'Человек',
      ability_score_increases: {},
      age: { mature_at: 18, max_lifespan: 90 },
      size: 'medium',
      speed: 30,
      languages: ['common'],
      proficiencies: { skills: [], weapons: [], tools: [], saves: [] },
      senses: {},
      traits: [],
      subraces: [],
      source_url: '',
      source_page: 78,
      srd_section: 'Character Origins / Species / Human',
    },
  ],
  classes: [
    {
      id: 'cleric',
      ruleset_id: 'srd-5.2.1',
      name_key: 'catalog.classes.cleric',
      name_en: 'Cleric',
      name_ru: 'Жрец',
      hit_die: 8,
      primary_ability: ['wis'],
      saving_throw_proficiencies: ['wis', 'cha'],
      armor_proficiencies: ['light', 'medium', 'shield'],
      weapon_proficiencies: ['simple'],
      tool_proficiencies: [],
      skill_proficiencies: {},
      starting_equipment: [],
      level_1_features: [],
      spellcasting: null,
      subclasses: [],
      source_url: '',
      source_page: 24,
      srd_section: 'Character Classes / Fighter',
    },
  ],
  backgrounds: [
    {
      id: 'soldier',
      ruleset_id: 'srd-5.2.1',
      name_key: 'catalog.backgrounds.soldier',
      name_en: 'Soldier',
      name_ru: 'Солдат',
      skill_proficiencies: ['athletics'],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: [],
      starting_gold: 10,
      ability_score_options: ['str', 'dex', 'con'],
      feat_id: 'savage-attacker',
      feature: { name_en: '', name_ru: '', description: '' },
      suggested_characteristics: {},
      source_url: '',
      source_page: 83,
      srd_section: 'Character Origins / Soldier',
    },
  ],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weaponProperties: [],
};

test('a complete campaign remains authoritative, restorable, and honest about bundled art', async ({
  page,
}) => {
  test.setTimeout(30_000);
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: null,
    active_provider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'http://127.0.0.1:31415/v1',
        apiKey: 'fixture-token',
        model: 'fixture-dm',
      },
      'local-mistralrs': null,
    },
    ui_language: 'en',
    narration_language: 'en',
    image_enabled: true,
    scene_transitions_enabled: false,
    active_campaign_id: CAMPAIGN_ID,
    active_session_id: SESSION_ID,
    char_creation_draft: {
      rulesetId: 'srd-5.2.1',
      classId: 'cleric',
      raceId: 'human',
      backgroundId: 'soldier',
      abilityMethod: '4d6_drop_lowest',
      abilities: { str: 16, dex: 14, con: 14, int: 10, wis: 12, cha: 10 },
      equipmentMode: 'gold',
      equipmentSlots: [],
      equipmentInventory: [{ id: 'mace', name: 'Mace', count: 1 }],
      spells: { cantrips: [], level1: ['cure-wounds'] },
      name: 'Aria',
      activeTab: 'review',
    },
  });

  for (const [path, body] of [
    ['races', compendium.races],
    ['classes', compendium.classes],
    ['backgrounds', compendium.backgrounds],
    ['spells', compendium.spells],
    ['equipment', compendium.equipment],
    ['feats', compendium.feats],
    ['weapon-properties', compendium.weaponProperties],
  ] as const) {
    await page.route(`**/srd/${path}`, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(body),
      });
    });
  }
  await page.route('**/settings/v2', async (route) => {
    await route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });

  characterId = null;
  let currentState: CampaignStateResponse = {
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign: {
      id: CAMPAIGN_ID,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: 'Aria Chronicle',
      premise: 'Moonlit Tavern',
      seed: { kind: 'collaborative', answers: {} },
      last_session_id: SESSION_ID,
      revision: 1,
      created_at: '2026-08-12T12:00:00Z',
      updated_at: '2026-08-12T12:00:00Z',
    },
    session_id: SESSION_ID,
    character_id: null,
    projection: {
      messages: [],
      character: {},
      combat: null,
      scene: { title: 'Opening table', subtitle: null, mode: 'exploration' },
      journal: [],
      npcs: [],
    },
  };
  await page.route(
    /^https?:\/\/[^/]+\/campaigns(?:\/[^/?]+(?:\/(?:state|character))?)?(?:\?|$)/,
    async (route) => {
      const path = new URL(route.request().url()).pathname;
      if (path.endsWith('/character') && route.request().method() === 'PUT') {
        const input = route.request().postDataJSON();
        expect(input.session_id).toBe(SESSION_ID);
        expect(input.expected_revision).toBe(currentState.campaign.revision);
        characterId = input.character_id;
        currentState.character_id = input.character_id;
        currentState.projection.character = { ...input.character, characterId: input.character_id };
        currentState.campaign.revision++;
      }
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(
          path === '/campaigns'
            ? [currentState.campaign]
            : path.endsWith('/state') || path.endsWith('/character')
              ? currentState
              : currentState.campaign,
        ),
      });
    },
  );
  const applyProjection = (value: ReturnType<typeof projection>) => {
    if (value.campaign_scope.revision < currentState.campaign.revision) return;
    currentState.campaign.revision = value.campaign_scope.revision;
    currentState.projection.combat = value;
    currentState.projection.character.hp = value.snapshot.combatants[0]?.current_hp;
  };
  let savedState: CampaignStateResponse | null = null;
  let quickSaveCreated = false;
  const saveSummary = {
    id: SAVE_ID,
    campaign_id: currentState.campaign.id,
    session_id: SESSION_ID,
    snapshot_id: SNAPSHOT_ID,
    revision: 11,
    kind: 'auto',
    title: 'Quick Save',
    summary: 'Moonlit Tavern',
    tag: 'exploration',
    created_at: '2026-08-12T12:00:00Z',
    turn_number: 5,
  };
  await page.route(/^https?:\/\/[^/]+\/sessions\//, async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname.endsWith('/messages')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ messages: [] }),
      });
      return;
    }
    if (url.pathname.endsWith('/saves/quick') && route.request().method() === 'POST') {
      quickSaveCreated = true;
      currentState.campaign.revision++;
      saveSummary.revision = currentState.campaign.revision;
      savedState = structuredClone(currentState);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ id: saveSummary.id, revision: saveSummary.revision }),
      });
      return;
    }
    if (url.pathname.endsWith('/saves')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(quickSaveCreated ? [saveSummary] : []),
      });
      return;
    }
    await route.fallback();
  });
  await page.route(/^https?:\/\/[^/]+\/saves\//, async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname.endsWith('/restore')) {
      expect(savedState).not.toBeNull();
      if (!savedState) throw new Error('missing checkpoint');
      const nextRevision = currentState.campaign.revision + 1;
      currentState = structuredClone(savedState);
      currentState.campaign.revision = nextRevision;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: SNAPSHOT_ID,
          campaign_id: CAMPAIGN_ID,
          session_id: SESSION_ID,
          revision: saveSummary.revision,
          current_revision: nextRevision,
          current_state: currentState,
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          projection: savedState.projection,
        }),
      });
      return;
    }
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        ...saveSummary,
        game_state: { schema_version: 2, pc_snapshot: { name: 'Aria', hp: 12, hpMax: 12 } },
      }),
    });
  });

  let agentTurn = 0;
  await page.route('**/agent/turn', async (route) => {
    agentTurn += 1;
    const input = route.request().postDataJSON();
    currentState.projection.messages.push({
      role: 'user',
      parts: [{ type: 'text', text: input.player_message }],
    });
    if (agentTurn === 1) {
      applyProjection(projection(0));
      currentState.projection.scene = {
        title: 'Moonlit Tavern',
        subtitle: null,
        mode: 'exploration',
      };
      currentState.projection.messages.push({
        role: 'assistant',
        content: 'Moonlight spills across the tavern floor.',
      });
      currentState.projection.npcs = [
        {
          id: 'mira',
          campaign_id: CAMPAIGN_ID,
          name: 'Mira Vale',
          role: 'innkeeper',
          disposition: 'friendly',
          trust: 0,
          facts: [{ text: 'Knows the old forest road.', created_at: '2026-08-12T12:00:00Z' }],
          updated_at: '2026-08-12T12:00:00Z',
        },
      ];
      currentState.projection.journal = [
        {
          id: 'journal-entry-1',
          campaign_id: CAMPAIGN_ID,
          chapter: 'The Moonlit Tavern',
          entry_html: '<p>A hidden path was revealed.</p>',
          created_at: '2026-08-12T12:00:00Z',
        },
      ];
      const body =
        event('text_delta', { text: 'Moonlight spills across the tavern floor.' }) +
        toolStart('scene-1', 'set_scene') +
        toolResult('scene-1', 'set_scene', { title: 'Moonlit Tavern' }, { ok: true }) +
        toolStart('illustration-1', 'generate_illustration') +
        event('image_generated', {
          tool_call_id: 'illustration-1',
          mime_type: 'image/png',
          image_b64: PIXEL_B64,
          kind: 'chat',
          source: 'bundled',
          asset_id: 'illustration-tavern',
        }) +
        toolResult('illustration-1', 'generate_illustration', {}, { ok: true }) +
        event('image_generated', {
          tool_call_id: 'map-1',
          mime_type: 'image/png',
          image_b64: PIXEL_B64,
          kind: 'map',
          source: 'bundled',
          asset_id: 'map-tavern-interior',
        }) +
        toolStart('map-1', 'generate_map') +
        toolResult('map-1', 'generate_map', {}, { ok: true }) +
        toolStart('npc-1', 'remember_npc') +
        toolResult(
          'npc-1',
          'remember_npc',
          {
            name: 'Mira Vale',
            role: 'innkeeper',
            disposition: 'friendly',
            fact: 'Knows the old forest road.',
          },
          { npc_id: 'mira' },
        ) +
        toolStart('journal-1', 'journal_append') +
        toolResult(
          'journal-1',
          'journal_append',
          { chapter: 'The Moonlit Tavern', entry_html: '<p>A hidden path was revealed.</p>' },
          { entry_id: 'journal-entry-1' },
        ) +
        toolStart('combat-start', 'start_combat') +
        toolResult('combat-start', 'start_combat', {}, { projection: projection(0) }) +
        event('agent_done', { total_rounds: 1 });
      await fulfillSse(route, body);
      return;
    }
    if (agentTurn === 2) {
      await new Promise((resolve) => setTimeout(resolve, 350));
      await fulfillSse(
        route,
        toolStart('old-receipt', 'combat_action') +
          toolResult(
            'old-receipt',
            'combat_action',
            { kind: 'weapon', target: GOBLIN_ID, weapon_id: 'mace' },
            { projection: projection(2, { heroX: 9, heroHp: 1, goblinHp: 7 }) },
          ) +
          event('text_delta', { text: 'The scout is wounded; Aria remains ready.' }) +
          event('agent_done', { total_rounds: 1 }),
      );
      currentState.projection.messages.push({
        role: 'assistant',
        content: 'The scout is wounded; Aria remains ready.',
      });
      return;
    }
    currentState.projection.scene = { title: 'Ashen Cellar', subtitle: null, mode: 'exploration' };
    currentState.projection.messages.push({
      role: 'assistant',
      content: 'You descend into the ashen cellar.',
    });
    await fulfillSse(
      route,
      toolStart('scene-2', 'set_scene') +
        toolResult('scene-2', 'set_scene', { title: 'Ashen Cellar' }, { ok: true }) +
        event('text_delta', { text: 'You descend into the ashen cellar.' }) +
        event('agent_done', { total_rounds: 1 }),
    );
  });

  let releaseMove: (() => void) | undefined;
  const moveReleased = new Promise<void>((resolve) => {
    releaseMove = resolve;
  });
  let capturedMove: unknown;
  let endTurnCount = 0;
  let attackCount = 0;
  await page.route('**/combat/action', async (route) => {
    const request = route.request().postDataJSON() as { action_type: string };
    if (request.action_type === 'move') {
      applyProjection(projection(1, { heroX: 2, heroHp: 8 }));
      capturedMove = request;
      await moveReleased;
      await fulfillSse(
        route,
        event('combat_projection', {
          projection: projection(1, { heroX: 2, heroHp: 8 }),
        }),
      );
      return;
    }
    if (request.action_type === 'spell') {
      expect(route.request().postDataJSON().args).toMatchObject({
        caster_id: HERO_ID,
        spell_id: 'cure-wounds',
        target_ids: [HERO_ID],
        slot_level: 1,
      });
      applyProjection(projection(2, { heroX: 2, heroHp: 10, heroAction: false }));
      await fulfillSse(
        route,
        event('combat_projection', {
          projection: projection(2, { heroX: 2, heroHp: 10, heroAction: false }),
        }),
      );
      return;
    }
    if (request.action_type === 'weapon_attack') {
      attackCount++;
      expect(route.request().postDataJSON().args).toMatchObject({
        attacker_id: HERO_ID,
        target_id: GOBLIN_ID,
        weapon_id: 'mace',
      });
      const next =
        attackCount === 1
          ? projection(4, { heroX: 2, heroHp: 10, heroAction: false, goblinHp: 2 })
          : projection(6, {
              active: false,
              current: null,
              heroX: 2,
              heroHp: 10,
              heroAction: false,
              goblinHp: 0,
            });
      applyProjection(next);
      await fulfillSse(route, event('combat_projection', { projection: next }));
      return;
    }
    expect(request.action_type).toBe('end_turn');
    endTurnCount++;
    const next = projection(endTurnCount === 1 ? 3 : 5, {
      heroX: 2,
      heroHp: 10,
      heroAction: true,
      goblinHp: endTurnCount === 1 ? 7 : 2,
      current: 'hero',
    });
    applyProjection(next);
    await fulfillSse(route, event('combat_projection', { projection: next }));
  });

  await test.step('create the player character through the production wizard', async () => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    await page.getByRole('button', { name: /Re-create character/i }).click();
    const wizard = page.locator('.dm-wizard');
    await expect(wizard.getByRole('heading', { name: 'Aria' })).toBeVisible();
    await wizard.getByRole('button', { name: /Begin Adventure/i }).click();
    await expect(
      page.getByRole('button', { name: /Open character sheet for Aria/i }),
    ).toBeVisible();
  });

  await test.step('play narration, tools, bundled illustration and tactical map', async () => {
    await sendChat(page, 'Begin the moonlit tavern encounter.');
    await expect(page.getByText('Moonlight spills across the tavern floor.')).toBeVisible();
    await expect(page.getByText('Moonlit Tavern', { exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Open painting: Scene painting', exact: true }).click();
    await expect(page.getByTestId('scene-art-preview').locator('img')).toBeVisible();
    await page.getByRole('button', { name: 'Close painting', exact: true }).click();
    await expect(page.locator('.dm-vtt-world img[data-asset-id="decorative-scene"]')).toHaveCount(
      0,
    );
    await expect(page.getByTestId('tool-call-card-illustration-1').getByRole('img')).toBeVisible();
    await expect(page.getByTestId('tool-call-card-map-1')).toHaveAttribute(
      'data-image-source',
      'bundled',
    );
    await expect(page.getByTestId('tool-call-card-illustration-1')).toHaveAttribute(
      'data-image-source',
      'bundled',
    );
    await expect(page.getByTestId(`combat-token-${HERO_ID}`)).toBeVisible();
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toBeVisible();
  });

  await test.step('move only after an authoritative revision and heal with a spell', async () => {
    const hero = page.getByTestId(`combat-token-${HERO_ID}`);
    await expect(hero).toHaveCSS('left', '0px');
    const box = await hero.boundingBox();
    expect(box).not.toBeNull();
    if (!box) throw new Error('hero token has no bounding box');
    const startX = box.x + box.width / 2;
    const startY = box.y + box.height / 2;
    await hero.dispatchEvent('pointerdown', {
      button: 0,
      pointerId: 1,
      clientX: startX,
      clientY: startY,
    });
    await hero.dispatchEvent('pointermove', {
      button: 0,
      pointerId: 1,
      clientX: startX + 60,
      clientY: startY,
    });
    await hero.dispatchEvent('pointerup', {
      button: 0,
      pointerId: 1,
      clientX: startX + 60,
      clientY: startY,
    });
    await expect
      .poll(() => capturedMove)
      .toMatchObject({
        expected_revision: 0,
        action_type: 'move',
        args: { combatant_id: HERO_ID, x: 2, y: 0 },
      });
    await expect(hero).toHaveCSS('left', '0px');
    releaseMove?.();
    await expect(hero).toHaveCSS('left', '60px');

    await page.getByTestId('action-btn-cast').click();
    await page.getByRole('button', { name: 'Confirm spell', exact: true }).click();
    await expect(page.getByLabel(/Aria 10\/10 HP/)).toBeVisible();
    await expect(page.getByTestId('action-btn-attack')).toBeDisabled();
  });

  await test.step('advance initiative, resolve an attack, ignore stale state, and end combat', async () => {
    await page.getByTestId('action-btn-end_turn').click();
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toBeVisible();
    await page.getByTestId('action-btn-cast').click();
    await expect(page.getByRole('dialog', { name: 'Cast a spell' })).toContainText(
      /1\s*\/\s*2 remaining/,
    );
    await page
      .getByRole('dialog', { name: 'Cast a spell' })
      .getByRole('button', { name: 'Cancel', exact: true })
      .click();
    await page.getByTestId('action-btn-attack').click();
    await page.getByRole('button', { name: 'Confirm attack', exact: true }).click();
    await expect(page.getByLabel(/Goblin Scout 2\/7 HP/)).toBeVisible();
    await expect(page.getByTestId(`combat-token-${HERO_ID}`)).toHaveCSS('left', '60px');
    await sendChat(page, 'Inspect the fight.');
    await expect(
      page.getByText('The scout is wounded; Aria remains ready.', { exact: true }),
    ).toBeVisible();
    await expect(page.getByTestId(`combat-token-${HERO_ID}`)).toHaveCSS('left', '60px');
    await expect(page.getByLabel(/Aria 10\/10 HP/)).toBeVisible();
    await expect(page.getByLabel(/Goblin Scout 2\/7 HP/)).toBeVisible();
    await page.getByTestId('action-btn-end_turn').click();
    await page.getByTestId('action-btn-attack').click();
    await page.getByRole('button', { name: 'Confirm attack', exact: true }).click();
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toHaveCount(0);
    await expect(page.getByRole('complementary', { name: 'Initiative' })).toHaveCount(0);
    await expect(page.getByTestId(`combat-token-${HERO_ID}`)).toHaveCount(0);
  });

  await test.step('inspect NPC and journal projections', async () => {
    await page.getByRole('button', { name: 'NPCs', exact: true }).click();
    await expect(page.getByRole('region', { name: 'NPC Memory' })).toContainText('Mira Vale');
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Journal', exact: true }).click();
    const journal = page.getByRole('region', { name: "Bard's Journal" });
    await expect(journal).toContainText('A hidden path was revealed.');
    await page.keyboard.press('Escape');
  });

  await test.step('quick-save, mutate, restore, and reload without persisted media', async () => {
    await page.keyboard.press('Control+S');
    await expect.poll(() => quickSaveCreated).toBe(true);
    await sendChat(page, 'Descend into the cellar.');
    await expect(page.getByText('Ashen Cellar', { exact: true })).toBeVisible();

    await page.keyboard.press('Control+Shift+S');
    const saves = page.locator('.dm-saves-overlay');
    await expect(saves).toBeVisible();
    await saves.getByRole('button', { name: /Load|Загрузить/ }).click();
    await expect(saves).toHaveCount(0);
    await expect(page.locator('.dm-scene-pill, .dm-table-scene')).toContainText('Moonlit Tavern');
    await expect(page.getByText('The scout is wounded; Aria remains ready.')).toBeVisible();
    await expect(page.locator('img[data-asset-id="decorative-scene"]')).toHaveCount(0);

    await page.reload();
    await expect(page.getByRole('button', { name: /Aria/i })).toBeVisible();
    await expect(page.locator('img[data-asset-id="decorative-scene"]')).toHaveCount(0);
    await expect(page.locator('[data-testid^="tool-call-card-"] img')).toHaveCount(0);
    await page.getByRole('button', { name: 'NPCs', exact: true }).click();
    await expect(page.getByRole('region', { name: 'NPC Memory' })).toContainText('Mira Vale');
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Journal', exact: true }).click();
    await expect(page.getByRole('region', { name: "Bard's Journal" })).toContainText(
      'A hidden path was revealed.',
    );
    await page.keyboard.press('Escape');
    const persisted = await page.evaluate(() => {
      const values: string[] = [];
      for (let index = 0; index < localStorage.length; index += 1) {
        const key = localStorage.key(index);
        if (key) values.push(localStorage.getItem(key) ?? '');
      }
      return values.join('\n');
    });
    expect(persisted).not.toContain('data:image');
    expect(persisted).not.toContain(PIXEL_B64);
  });
});
