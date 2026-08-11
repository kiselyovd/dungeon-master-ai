import { describe, expect, it } from 'vitest';
import type { RestoreSaveResponse, SaveRow } from '../../../../api/saves';
import { buildRestoredSession, RestoreValidationError } from '../buildRestoredSession';

const row: SaveRow = {
  id: 'save-1',
  session_id: 'session-1',
  kind: 'manual',
  title: 'Checkpoint',
  summary: 'Before the door',
  tag: 'exploration',
  created_at: '2026-08-11T00:00:00Z',
  turn_number: 4,
  game_state: { schema_version: 2, pc_snapshot: { name: 'Restored Hero', hp: 9 } },
};

function restored(): RestoreSaveResponse {
  return {
    game_state: {
      schema_version: 2,
      scene: { title: 'Hall of Echoes', subtitle: null, mode: 'combat' },
      combat: {
        active: true,
        encounter_id: 'enc-1',
        round: 2,
        current_turn_id: 'hero',
        initiative: ['hero'],
        tokens: [
          {
            id: 'hero',
            name: 'Restored Hero',
            hp: 9,
            max_hp: 12,
            ac: 16,
            x: 2,
            y: 3,
            conditions: [],
            resistances: [],
            immunities: [],
            vulnerabilities: [],
          },
        ],
      },
    },
    messages: [
      { role: 'assistant_with_tool_calls', content: 'hidden', tool_calls: [] },
      { role: 'user', parts: [{ type: 'text', text: 'Open the door' }] },
      { role: 'tool_result', content: 'hidden' },
      { role: 'assistant', content: 'The hall opens.' },
    ],
  };
}

describe('buildRestoredSession', () => {
  it('builds one complete v2 projection and filters non-renderable messages', () => {
    const projection = buildRestoredSession({
      saveId: 'save-1',
      campaignId: 'campaign-1',
      row,
      restored: restored(),
    });

    expect(projection).toMatchObject({
      campaignId: 'campaign-1',
      sessionId: 'session-1',
      pc: { name: 'Restored Hero', hp: 9 },
      scene: { name: 'Hall of Echoes', stepCounter: 0 },
      combat: { encounter_id: 'enc-1', round: 2 },
    });
    expect(projection.messages.map(({ role, content }) => ({ role, content }))).toEqual([
      { role: 'user', content: 'Open the door' },
      { role: 'assistant', content: 'The hall opens.' },
    ]);
  });

  it('rejects unsupported versions before producing a projection', () => {
    const invalid = restored();
    invalid.game_state = { schema_version: 1 };
    expect(() =>
      buildRestoredSession({
        saveId: 'save-1',
        campaignId: 'campaign-1',
        row,
        restored: invalid,
      }),
    ).toThrowError(new RestoreValidationError('restore_version_unsupported'));
  });

  it('rejects malformed combat instead of partially accepting it', () => {
    const invalid = restored();
    invalid.game_state = {
      schema_version: 2,
      combat: { active: true, encounter_id: 'enc-1', initiative: [], tokens: [] },
    };
    expect(() =>
      buildRestoredSession({
        saveId: 'save-1',
        campaignId: 'campaign-1',
        row,
        restored: invalid,
      }),
    ).toThrowError(RestoreValidationError);
  });
});
