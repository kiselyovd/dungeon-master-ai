import { expect, test } from '@playwright/test';
import { mockTauri } from './fixtures/tauri-mock';

const campaignId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const sessionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const characterId = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
const adventure = (location: 'town' | 'quay') => ({
  state: {
    package_id: 'bell-beneath-water',
    location,
    facts: ['ferry_route'],
    ending: null,
    closed: false,
    history: location === 'quay' ? [{ kind: 'move', location: 'quay' }] : [],
  },
  available_actions: [{ kind: 'move', location: location === 'town' ? 'quay' : 'town' }],
});
test('confirmed adventure tool outcome updates the production root while narration continues', async ({
  page,
}) => {
  await mockTauri(page, {
    onboarding_completed: true,
    ui_language: 'en',
    hero_class: 'fighter',
    active_campaign_id: campaignId,
    active_session_id: sessionId,
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
        session_id: sessionId,
        character_id: characterId,
        campaign: {
          id: campaignId,
          last_session_id: sessionId,
          revision: 2,
          schema_version: 1,
          ruleset_id: 'srd-5.2.1',
          title: 'Bell',
          premise: 'Ferry',
          seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
          created_at: '2026-10-06T00:00:00.000Z',
          updated_at: '2026-10-06T00:00:00.000Z',
        },
        projection: {
          character: {
            characterId,
            name: 'Mira',
            heroClass: 'fighter',
            level: 1,
            hp: 12,
            hpMax: 12,
            ac: 14,
            inventory: [],
          },
          messages: [],
          combat: null,
          scene: null,
          journal: [],
          npcs: [],
          adventure: adventure('town'),
        },
      }),
    }),
  );
  await page.route('**/sessions/*/messages', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: '{"messages":[]}' }),
  );
  await page.route('**/agent/turn', (route) => {
    const outcome = {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      campaign_id: campaignId,
      session_id: sessionId,
      character_id: characterId,
      revision: 3,
      adventure: adventure('quay'),
      inventory: [],
    };
    const frames = [
      ['tool_call_start', { id: 'call-1', tool_name: 'adventure_action', round: 1 }],
      [
        'tool_call_result',
        {
          id: 'call-1',
          tool_name: 'adventure_action',
          args: { action: { kind: 'move', location: 'quay' } },
          result: outcome,
          is_error: false,
          handled_by: 'engine',
          round: 1,
        },
      ],
      [
        'text_delta',
        { text: 'You step onto the wet planks. The upper path leads toward the archive.' },
      ],
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
  await expect(page.getByRole('button', { name: 'Adventure', exact: true })).toBeVisible();
  await page.getByRole('textbox', { name: 'What do you do?' }).fill('I go to the old quay');
  await page.getByRole('button', { name: 'Send', exact: true }).click();
  await expect(
    page.getByText('You step onto the wet planks. The upper path leads toward the archive.', {
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByText('I go to the old quay', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Adventure', exact: true }).click();
  await expect(
    page.getByRole('dialog').getByRole('heading', { name: 'Old quay', exact: true }),
  ).toBeVisible();
  await page.screenshot({ path: '.artifacts/ai-adventure/chat-outcome.png', fullPage: true });
});
