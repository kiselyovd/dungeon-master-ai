import { beforeEach, describe, expect, it } from 'vitest';
import type { CombatToken } from '../../state/combat';
import { useStore } from '../../state/useStore';
import { combatToolHandlers } from '../useCombatToolHandlers';

function makeToken(overrides: Partial<CombatToken> = {}): CombatToken {
  return {
    id: 'tok-1',
    name: 'Hero',
    hp: 10,
    maxHp: 10,
    ac: 14,
    x: 0,
    y: 0,
    conditions: [],
    ...overrides,
  };
}

// The exact set of backend combat tools that mutate the VTT. Must stay in
// sync with crates/app-server/src/agent/tools.rs. A frontend handler is
// required for each; no handler may exist for a tool the backend never emits.
const BACKEND_COMBAT_TOOLS = [
  'start_combat',
  'end_combat',
  'apply_damage',
  'apply_healing',
  'add_token',
  'update_token',
  'remove_token',
];

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
});

describe('combatToolHandlers contract', () => {
  it('has exactly one handler per backend combat tool, and no extras', () => {
    expect(Object.keys(combatToolHandlers).sort()).toEqual([...BACKEND_COMBAT_TOOLS].sort());
  });
});

describe('start_combat handler', () => {
  it('builds tokens from initiative_entries, not args.tokens', () => {
    combatToolHandlers.start_combat?.(
      {
        initiative_entries: [
          { id: 'goblin-1', name: 'Goblin', roll: 14, dex_mod: 2, hp: 7, max_hp: 7, ac: 15 },
          { id: 'pc-1', name: 'Hero', roll: 18, dex_mod: 3, hp: 20, max_hp: 20, ac: 16 },
        ],
      },
      { encounter_id: 'enc-1' },
      useStore,
    );
    const c = useStore.getState().combat;
    expect(c.active).toBe(true);
    expect(c.tokens.map((t) => t.id)).toEqual(['goblin-1', 'pc-1']);
    expect(c.tokens[0]?.maxHp).toBe(7);
    expect(c.tokens[1]?.ac).toBe(16);
  });
});

describe('damage and healing handlers', () => {
  it('apply_damage reduces token hp', () => {
    useStore.getState().combat.startCombat('enc-1', [makeToken()]);
    combatToolHandlers.apply_damage?.({ token_id: 'tok-1', amount: 4 }, {}, useStore);
    expect(useStore.getState().combat.tokens[0]?.hp).toBe(6);
  });

  it('apply_healing increases token hp clamped at maxHp', () => {
    useStore.getState().combat.startCombat('enc-2', [makeToken({ hp: 5 })]);
    combatToolHandlers.apply_healing?.({ token_id: 'tok-1', amount: 10 }, {}, useStore);
    expect(useStore.getState().combat.tokens[0]?.hp).toBe(10);
  });
});

describe('token handlers', () => {
  it('add_token appends, update_token patches, remove_token drops', () => {
    useStore.getState().combat.startCombat('enc-1', [makeToken({ id: 'pc-1' })]);

    combatToolHandlers.add_token?.(
      { id: 'orc-1', name: 'Orc', x: 3, y: 4, hp: 15, max_hp: 15, ac: 13 },
      {},
      useStore,
    );
    expect(useStore.getState().combat.tokens.find((t) => t.id === 'orc-1')?.x).toBe(3);

    combatToolHandlers.update_token?.({ id: 'orc-1', hp: 5, conditions: ['prone'] }, {}, useStore);
    const orc = useStore.getState().combat.tokens.find((t) => t.id === 'orc-1');
    expect(orc?.hp).toBe(5);
    expect(orc?.conditions).toEqual(['prone']);

    combatToolHandlers.remove_token?.({ id: 'orc-1' }, {}, useStore);
    expect(useStore.getState().combat.tokens.find((t) => t.id === 'orc-1')).toBeUndefined();
  });
});

describe('end_combat handler', () => {
  it('clears combat state', () => {
    useStore.getState().combat.startCombat('enc-1', [makeToken()]);
    combatToolHandlers.end_combat?.({}, {}, useStore);
    expect(useStore.getState().combat.active).toBe(false);
    expect(useStore.getState().combat.tokens).toHaveLength(0);
  });
});
