import { expect, it } from 'vitest';
import type { RestoreSaveResponse } from '../../api/saves';
import { buildRestoredSession } from '../campaignProjection';

it('applies the full current server projection and its new revision after restore', () => {
  const oldProjection = {
    messages: [],
    character: { name: 'Old hero', hp: 1 },
    combat: null,
    scene: null,
    journal: [],
    npcs: [],
  };
  const campaignId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const sessionId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
  const restored = {
    id: 'save',
    campaign_id: campaignId,
    session_id: sessionId,
    revision: 4,
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    projection: oldProjection,
    current_state: {
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      session_id: sessionId,
      character_id: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc',
      campaign: {
        id: campaignId,
        last_session_id: sessionId,
        revision: 8,
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        title: 'Bell',
        premise: '',
        seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
        created_at: '2026-10-06',
        updated_at: '2026-10-06',
      },
      projection: {
        ...oldProjection,
        character: {
          name: 'Confirmed hero',
          heroClass: 'fighter',
          hp: 8,
          hpMax: 12,
          level: 1,
          inventory: [],
        },
      },
    },
  } as RestoreSaveResponse;
  const result = buildRestoredSession({
    saveId: 'save',
    campaignId,
    sessionId,
    expectedRevision: 7,
    restored,
  });
  expect(result.pc.name).toBe('Confirmed hero');
  expect(result.revision).toBe(8);
  expect(result.campaign?.revision).toBe(8);
});
