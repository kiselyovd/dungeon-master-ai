import { afterEach, expect, it, vi } from 'vitest';
import { setBackendPortForTesting } from '../../../../api/client';
import { sendCombatCommand } from '../commands';

afterEach(() => {
  vi.unstubAllGlobals();
  setBackendPortForTesting(null);
});
it('sends only a weapon intention rather than client damage or attack modifiers', async () => {
  setBackendPortForTesting(45678);
  const projection = {
    schema_version: 1,
    encounter_id: 'enc',
    revision: 1,
    snapshot: { active: true, round: 1, current_combatant: 'hero', initiative: [], combatants: [] },
    events: [],
  };
  const fetch = vi.fn(async (_url: unknown, _options: RequestInit) => ({
    ok: true,
    body: {},
    arrayBuffer: async () =>
      new TextEncoder().encode(
        `event: combat_projection\ndata: ${JSON.stringify({ projection })}\n\n`,
      ).buffer,
  }));
  vi.stubGlobal('fetch', fetch);
  await sendCombatCommand({
    encounterId: 'enc',
    revision: 0,
    commandId: 'id',
    command: {
      kind: 'attack',
      attackerId: 'hero',
      targetId: 'enemy',
      attackModifier: 999,
      damageDice: '100d100',
      damageType: 'fire',
    },
  });
  const options = fetch.mock.calls[0]?.[1] as RequestInit;
  const body = JSON.parse(String(options.body));
  expect(body.action_type).toBe('weapon_attack');
  expect(body.args).toEqual({ attacker_id: 'hero', target_id: 'enemy' });
});
