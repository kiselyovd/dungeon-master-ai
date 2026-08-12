import { expect, type Page, type Route, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const CAMPAIGN_ID = '00000000-0000-4000-8000-000000000001';
const SESSION_ID = '00000000-0000-4000-8000-000000000002';
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
    encounter_id: 'encounter-campaign',
    revision,
    snapshot: {
      active,
      round: revision >= 3 ? 2 : 1,
      current_combatant: current,
      initiative: [
        { id: 'hero', roll: 18, dex_tiebreak: 2 },
        { id: 'goblin', roll: 12, dex_tiebreak: 1 },
      ],
      combatants: [
        {
          id: 'hero',
          name: 'Aria',
          max_hp: 12,
          current_hp: heroHp,
          temp_hp: 0,
          ac: 14,
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
          id: 'goblin',
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
          position: { x: 4, y: 0 },
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
      srd_section: '',
    },
  ],
  classes: [
    {
      id: 'fighter',
      name_en: 'Fighter',
      name_ru: 'Воин',
      hit_die: 10,
      primary_ability: ['str'],
      saving_throw_proficiencies: ['str', 'con'],
      armor_proficiencies: [],
      weapon_proficiencies: [],
      tool_proficiencies: [],
      skill_proficiencies: {},
      starting_equipment: [],
      level_1_features: [],
      spellcasting: null,
      subclasses: [],
      source_url: '',
    },
  ],
  backgrounds: [
    {
      id: 'soldier',
      name_en: 'Soldier',
      name_ru: 'Солдат',
      skill_proficiencies: ['athletics'],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: [],
      starting_gold: 10,
      feature: { name_en: '', name_ru: '', description: '' },
      suggested_characteristics: {},
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
  test.setTimeout(90_000);
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
      classId: 'fighter',
      raceId: 'human',
      backgroundId: 'soldier',
      abilityMethod: 'standard_array',
      abilities: { str: 16, dex: 14, con: 14, int: 10, wis: 12, cha: 10 },
      equipmentMode: 'gold',
      equipmentSlots: [],
      equipmentInventory: [],
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

  let quickSaveCreated = false;
  const saveSummary = {
    id: 'save-quick',
    session_id: SESSION_ID,
    kind: 'checkpoint',
    title: 'Quick Save',
    summary: 'Moonlit Tavern',
    tag: 'exploration',
    created_at: '2026-08-12T12:00:00Z',
    turn_number: 5,
  };
  await page.route('http://127.0.0.1:31415/sessions/**', async (route) => {
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
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ id: saveSummary.id }),
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
  await page.route('http://127.0.0.1:31415/saves/**', async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname.endsWith('/restore')) {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          game_state: {
            schema_version: 2,
            scene: { title: 'Moonlit Tavern', subtitle: null, mode: 'exploration' },
          },
          messages: [
            { role: 'user', parts: [{ type: 'text', text: 'We return to safety.' }] },
            { role: 'assistant', content: 'The hearth burns softly.' },
          ],
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
    if (agentTurn === 1) {
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
      await fulfillSse(
        route,
        event('text_delta', { text: 'The scout lunges, and Aria regains the initiative.' }) +
          toolStart('turn-hero', 'start_combat') +
          toolResult(
            'turn-hero',
            'start_combat',
            {},
            {
              projection: projection(4, { heroX: 2, heroHp: 12, current: 'hero' }),
            },
          ) +
          event('agent_done', { total_rounds: 1 }),
      );
      return;
    }
    if (agentTurn === 3) {
      await fulfillSse(
        route,
        toolStart('attack-stale', 'apply_damage') +
          toolResult(
            'attack-stale',
            'apply_damage',
            {},
            {
              projection: projection(2, { heroX: 9, heroHp: 1, goblinHp: 7 }),
            },
          ) +
          toolStart('attack-hit', 'apply_damage') +
          toolResult(
            'attack-hit',
            'apply_damage',
            {},
            {
              projection: projection(5, {
                heroX: 2,
                heroHp: 12,
                heroAction: false,
                goblinHp: 2,
              }),
            },
          ) +
          event('text_delta', { text: 'The blade strikes true.' }) +
          event('agent_done', { total_rounds: 1 }),
      );
      return;
    }
    if (agentTurn === 4) {
      await fulfillSse(
        route,
        toolStart('combat-end', 'end_combat') +
          toolResult(
            'combat-end',
            'end_combat',
            {},
            {
              projection: projection(7, {
                active: false,
                current: null,
                heroX: 2,
                heroHp: 12,
                goblinHp: 0,
              }),
            },
          ) +
          event('text_delta', { text: 'The tavern falls quiet once more.' }) +
          event('agent_done', { total_rounds: 1 }),
      );
      return;
    }
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
  await page.route('**/combat/action', async (route) => {
    const request = route.request().postDataJSON() as { action_type: string };
    if (request.action_type === 'move') {
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
    if (request.action_type === 'cast') {
      await fulfillSse(
        route,
        event('combat_projection', {
          projection: projection(2, { heroX: 2, heroHp: 12, heroAction: false }),
        }),
      );
      return;
    }
    endTurnCount += 1;
    await fulfillSse(
      route,
      event('combat_projection', {
        projection: projection(endTurnCount === 1 ? 3 : 6, {
          heroX: 2,
          heroHp: 12,
          heroAction: false,
          goblinHp: endTurnCount === 1 ? 7 : 2,
          current: 'goblin',
        }),
      }),
    );
  });

  const staleWarnings: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'warning' && message.text().includes('stale_combat_projection')) {
      staleWarnings.push(message.text());
    }
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
    await expect(page.getByTestId('dm-vtt-map-bg')).toBeVisible();
    await expect(page.getByTestId('tool-call-card-illustration-1').getByRole('img')).toBeVisible();
    await expect(page.getByText('From the built-in collection')).toHaveCount(2);
    await expect(page.getByTestId('combat-token-hero')).toBeVisible();
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toBeVisible();

    await page.getByRole('button', { name: 'Settings', exact: true }).click();
    let settings = page.getByRole('dialog', { name: 'Settings' });
    await settings.getByRole('tab', { name: 'Behavior', exact: true }).click();
    await settings.getByLabel('UI language').click();
    await settings.getByRole('option', { name: 'Russian', exact: true }).click();
    await settings.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Из встроенной коллекции')).toHaveCount(2);

    await page.getByRole('button', { name: 'Настройки', exact: true }).click();
    settings = page.getByRole('dialog', { name: 'Настройки' });
    await settings.getByRole('tab').last().click();
    await settings.getByLabel('Язык интерфейса').selectOption('en');
    await settings.getByRole('button', { name: 'Сохранить', exact: true }).click();
    await expect(page.getByText('From the built-in collection')).toHaveCount(2);
  });

  await test.step('move only after an authoritative revision and heal with a spell', async () => {
    const hero = page.getByTestId('combat-token-hero');
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
        args: { combatant_id: 'hero', x: 2, y: 0 },
      });
    await expect(hero).toHaveCSS('left', '0px');
    releaseMove?.();
    await expect(hero).toHaveCSS('left', '60px');

    await page.getByTestId('action-btn-cast').click();
    await expect(page.getByLabel(/Aria 12\/12 HP/)).toBeVisible();
    await expect(page.getByTestId('action-btn-attack')).toBeDisabled();
  });

  await test.step('advance initiative, resolve an attack, ignore stale state, and end combat', async () => {
    await page.getByTestId('action-btn-end_turn').click();
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toHaveCount(0);
    await sendChat(page, 'Resolve the goblin turn.');
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toBeVisible();
    await page.getByTestId('action-btn-attack').click();
    await expect(page.getByLabel(/Goblin Scout 2\/7 HP/)).toBeVisible();
    await expect(page.getByTestId('combat-token-hero')).toHaveCSS('left', '60px');
    await expect.poll(() => staleWarnings.length).toBeGreaterThan(0);
    await page.getByTestId('action-btn-end_turn').click();
    await sendChat(page, 'End the defeated encounter.');
    await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toHaveCount(0);
    await expect(page.getByRole('complementary', { name: 'Initiative' })).toHaveCount(0);
    await expect(page.getByTestId('combat-token-hero')).not.toHaveAttribute('data-active', 'true');
  });

  await test.step('inspect NPC and journal projections', async () => {
    await page.getByRole('button', { name: 'NPCs', exact: true }).click();
    await expect(page.getByRole('dialog', { name: 'NPC Memory' })).toContainText('Mira Vale');
    await page.getByRole('button', { name: 'Close NPC memory' }).click();
    await page.getByRole('button', { name: 'Journal', exact: true }).click();
    const journal = page.getByRole('dialog', { name: "Bard's Journal" });
    await expect(journal).toContainText('A hidden path was revealed.');
    await page.getByRole('button', { name: 'Close journal' }).click();
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
    await expect(page.locator('.dm-scene-pill')).toContainText('Moonlit Tavern');
    await expect(page.getByText('The hearth burns softly.')).toBeVisible();
    await expect(page.getByTestId('dm-vtt-map-bg')).toHaveCount(0);

    await page.reload();
    await expect(page.getByRole('button', { name: /Aria/i })).toBeVisible();
    await expect(page.getByTestId('dm-vtt-map-bg')).toHaveCount(0);
    await expect(page.locator('[data-testid^="tool-call-card-"] img')).toHaveCount(0);
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
