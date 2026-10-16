import { expect, it } from 'vitest';
import { npcAppearance } from '../npcAppearance';
import { useStore } from '../useStore';

it('keeps an acknowledged portrait ID despite role or catalog changes', async () => {
  const scope = crypto.randomUUID();
  expect(await npcAppearance(scope, 'npc.innkeeper.legacy.v1')).toBe('npc.innkeeper.legacy.v1');
  expect(await npcAppearance(scope, 'npc.mage.new.v2')).toBe('npc.innkeeper.legacy.v1');
});
it('equal NPC display names keep distinct records and never receive an ambiguous fact', () => {
  const npc = {
    campaign_id: 'campaign',
    name: 'Mira',
    role: '',
    disposition: 'unknown' as const,
    trust: 0,
    facts: [],
    updated_at: '2026-10-13T00:00:00Z',
  };
  useStore.getState().npcs.setNpcs([
    { ...npc, id: 'one' },
    { ...npc, id: 'two' },
  ]);
  useStore.getState().npcs.addFact('Mira', { text: 'Ambiguous', created_at: 'now' });
  expect(Object.keys(useStore.getState().npcs.records)).toEqual(['one', 'two']);
  expect(useStore.getState().npcs.records.one?.facts).toEqual([]);
  useStore.getState().npcs.addFact('two', { text: 'Confirmed second identity', created_at: 'now' });
  expect(useStore.getState().npcs.records.two?.facts).toHaveLength(1);
});

it('campaign restoration retains two same-name NPCs under their canonical IDs', () => {
  const npc = {
    campaign_id: 'campaign',
    name: 'Mira',
    role: '',
    disposition: 'neutral' as const,
    trust: 0,
    facts: [],
    updated_at: 'now',
  };
  useStore.getState().restoration.apply({
    campaignId: 'campaign',
    sessionId: 'session',
    revision: 1,
    messages: [],
    pc: {},
    scene: null,
    combat: null,
    journal: [],
    npcs: [
      { ...npc, id: 'first' },
      { ...npc, id: 'second' },
    ],
  });
  expect(Object.keys(useStore.getState().npcs.records)).toEqual(['first', 'second']);
});
