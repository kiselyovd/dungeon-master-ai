import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const projection = (revision: number, heroX: number) => ({
  schema_version: 1,
  encounter_id: 'encounter-e2e',
  revision,
  snapshot: {
    active: true,
    round: 1,
    current_combatant: 'hero',
    initiative: [
      { id: 'hero', roll: 18, dex_tiebreak: 2 },
      { id: 'goblin', roll: 12, dex_tiebreak: 1 },
    ],
    combatants: [
      {
        id: 'hero',
        name: 'Hero',
        max_hp: 12,
        current_hp: 12,
        temp_hp: 0,
        ac: 16,
        speed_ft: 30,
        initiative_roll: 18,
        dex_mod: 2,
        conditions: [],
        budget: {
          action: true,
          bonus_action: true,
          reaction: true,
          movement_ft: 30 - heroX * 5,
        },
        is_dead: false,
        position: { x: heroX, y: 0 },
      },
      {
        id: 'goblin',
        name: 'Goblin',
        max_hp: 7,
        current_hp: 7,
        temp_hp: 0,
        ac: 13,
        speed_ft: 30,
        initiative_roll: 12,
        dex_mod: 1,
        conditions: [],
        budget: {
          action: true,
          bonus_action: true,
          reaction: true,
          movement_ft: 30,
        },
        is_dead: false,
        position: { x: 4, y: 0 },
      },
    ],
  },
  events: [],
});

test('token movement remains a request until a newer server projection arrives', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    hero_class: 'fighter',
    active_provider: 'local-mistralrs',
    pc: {
      heroClass: 'fighter',
      name: 'Hero',
      race: 'Human',
      subclass: null,
      background: 'Soldier',
      alignment: 'Neutral',
      level: 1,
      experience: 0,
      experienceNext: 300,
      hp: 12,
      hpMax: 12,
      ac: 16,
      initiative: 2,
      speedFt: 30,
      proficiencyBonus: 2,
      abilities: { str: 16, dex: 14, con: 14, int: 10, wis: 12, cha: 10 },
      savingThrowProfs: { str: true, con: true },
      skillProfs: { athletics: true },
      inventory: [],
      portraitUrl: null,
    },
  });
  await page.route('**/sessions/**', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ messages: [] }),
    });
  });
  await page.route('**/agent/turn', async (route) => {
    const body = [
      'event: tool_call_start',
      'data: {"id":"combat-start","tool_name":"start_combat","round":1}',
      '',
      'event: tool_call_result',
      `data: ${JSON.stringify({
        id: 'combat-start',
        tool_name: 'start_combat',
        args: {},
        result: { projection: projection(0, 0) },
        is_error: false,
        round: 1,
        handled_by: 'engine',
      })}`,
      '',
      'event: agent_done',
      'data: {"total_rounds":1}',
      '',
      '',
    ].join('\n');
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });

  let releaseProjection: (() => void) | undefined;
  const projectionReleased = new Promise<void>((resolve) => {
    releaseProjection = resolve;
  });
  let capturedAction: unknown;
  await page.route('**/combat/action', async (route) => {
    capturedAction = route.request().postDataJSON();
    await projectionReleased;
    const body = `event: combat_projection\ndata: ${JSON.stringify({
      projection: projection(1, 2),
    })}\n\n`;
    await route.fulfill({
      status: 200,
      headers: { 'content-type': 'text/event-stream' },
      body,
    });
  });

  await page.goto('/');
  await page.getByPlaceholder(/what do you do/i).fill('start combat');
  await page.getByRole('button', { name: 'Send', exact: true }).click();

  const hero = page.getByTestId('combat-token-hero');
  await expect(hero).toBeVisible();
  await expect(page.getByRole('toolbar', { name: 'Combat actions' })).toBeVisible();
  await expect(hero).toHaveCSS('left', '0px');

  const box = await hero.boundingBox();
  expect(box).not.toBeNull();
  if (!box) return;
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

  await expect.poll(() => capturedAction).toBeTruthy();
  expect(capturedAction).toMatchObject({
    encounter_id: 'encounter-e2e',
    expected_revision: 0,
    action_type: 'move',
    args: { combatant_id: 'hero', x: 2, y: 0 },
  });
  await expect(hero).toHaveCSS('left', '0px');
  await expect(page.getByRole('button', { name: /^end turn/i })).toBeDisabled();

  releaseProjection?.();
  await expect(hero).toHaveCSS('left', '60px');
  await expect(page.getByRole('button', { name: /^end turn/i })).toBeEnabled();
});
