import { describe, expect, it } from 'vitest';
import type { RestoreSaveResponse } from '../../../../api/saves';
import { buildRestoredSession, RestoreValidationError } from '../buildRestoredSession';

function restored(): RestoreSaveResponse {
  return {
    id: 'snapshot-1',
    campaign_id: 'campaign-1',
    session_id: 'session-1',
    revision: 4,
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    projection: {
      character: { name: 'Restored Hero', hp: 9 },
      scene: { title: 'Hall of Echoes', subtitle: null, mode: 'combat' },
      combat: {
        schema_version: 1,
        encounter_id: 'enc-1',
        revision: 4,
        snapshot: {
          active: true,
          round: 2,
          current_combatant: 'hero',
          initiative: [{ id: 'hero', roll: 12, dex_tiebreak: 2 }],
          combatants: [
            {
              id: 'hero',
              name: 'Restored Hero',
              max_hp: 12,
              current_hp: 9,
              temp_hp: 0,
              ac: 16,
              speed_ft: 30,
              initiative_roll: 12,
              dex_mod: 2,
              conditions: [],
              budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
              is_dead: false,
              position: { x: 2, y: 3 },
            },
          ],
        },
        events: [],
      },
      messages: [
        { role: 'assistant_with_tool_calls', content: 'hidden', tool_calls: [] },
        { role: 'user', parts: [{ type: 'text', text: 'Open the door' }] },
        { role: 'tool_result', content: 'hidden' },
        { role: 'assistant', content: 'The hall opens.' },
      ],
      journal: [],
      npcs: [],
    },
  };
}

describe('buildRestoredSession', () => {
  it('builds one complete v2 projection and filters non-renderable messages', () => {
    const projection = buildRestoredSession({
      saveId: 'save-1',
      campaignId: 'campaign-1',
      sessionId: 'session-1',
      expectedRevision: 4,
      restored: restored(),
    });

    expect(projection).toMatchObject({
      campaignId: 'campaign-1',
      sessionId: 'session-1',
      pc: { name: 'Restored Hero', hp: 9 },
      scene: { name: 'Hall of Echoes', stepCounter: 0 },
      combat: { encounter_id: 'enc-1', snapshot: { round: 2 } },
    });
    expect(projection.messages.map(({ role, content }) => ({ role, content }))).toEqual([
      { role: 'user', content: 'Open the door' },
      { role: 'assistant', content: 'The hall opens.' },
    ]);
  });

  it('rejects unsupported versions before producing a projection', () => {
    const invalid = restored();
    (invalid as { schema_version: number }).schema_version = 2;
    expect(() =>
      buildRestoredSession({
        saveId: 'save-1',
        campaignId: 'campaign-1',
        sessionId: 'session-1',
        expectedRevision: 4,
        restored: invalid,
      }),
    ).toThrowError(new RestoreValidationError('restore_version_unsupported'));
  });

  it('rejects malformed combat instead of partially accepting it', () => {
    const invalid = restored();
    invalid.projection.combat = { schema_version: 1, encounter_id: 'enc-1', revision: 4 };
    expect(() =>
      buildRestoredSession({
        saveId: 'save-1',
        campaignId: 'campaign-1',
        sessionId: 'session-1',
        expectedRevision: 4,
        restored: invalid,
      }),
    ).toThrowError(RestoreValidationError);
  });
});
