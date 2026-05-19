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

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
});

describe('combatToolHandlers', () => {
  it('apply_damage reduces token hp', () => {
    useStore.getState().combat.startCombat('enc-1', [makeToken()]);
    combatToolHandlers['apply_damage']?.({ token_id: 'tok-1', amount: 4 }, {}, useStore);
    expect(useStore.getState().combat.tokens[0]?.hp).toBe(6);
  });

  it('apply_healing increases token hp clamped at maxHp', () => {
    useStore.getState().combat.startCombat('enc-2', [makeToken({ hp: 5 })]);
    combatToolHandlers['apply_healing']?.({ token_id: 'tok-1', amount: 10 }, {}, useStore);
    expect(useStore.getState().combat.tokens[0]?.hp).toBe(10);
  });

  it('add_condition appends to conditions array', () => {
    useStore.getState().combat.startCombat('enc-3', [makeToken()]);
    combatToolHandlers['add_condition']?.({ token_id: 'tok-1', condition: 'prone' }, {}, useStore);
    expect(useStore.getState().combat.tokens[0]?.conditions).toContain('prone');
  });

  it('remove_condition removes from conditions array', () => {
    useStore.getState().combat.startCombat('enc-4', [makeToken({ conditions: ['prone'] })]);
    combatToolHandlers['remove_condition']?.(
      { token_id: 'tok-1', condition: 'prone' },
      {},
      useStore,
    );
    expect(useStore.getState().combat.tokens[0]?.conditions).not.toContain('prone');
  });

  it('set_current_turn updates currentTurnId', () => {
    useStore
      .getState()
      .combat.startCombat('enc-5', [
        makeToken({ id: 'a' }),
        makeToken({ id: 'b', name: 'Goblin' }),
      ]);
    combatToolHandlers['set_current_turn']?.({ token_id: 'b' }, {}, useStore);
    expect(useStore.getState().combat.currentTurnId).toBe('b');
  });

  it('move_token updates grid position', () => {
    useStore.getState().combat.startCombat('enc-6', [makeToken()]);
    combatToolHandlers['move_token']?.({ token_id: 'tok-1', x: 3, y: 4 }, {}, useStore);
    const tok = useStore.getState().combat.tokens[0];
    expect(tok?.x).toBe(3);
    expect(tok?.y).toBe(4);
  });

  it('start_combat activates combat with provided tokens', () => {
    combatToolHandlers['start_combat']?.(
      {
        encounter_id: 'enc-7',
        tokens: [
          { id: 'w', name: 'Warrior', hp: 20, max_hp: 20, ac: 16, x: 0, y: 0, conditions: [] },
        ],
      },
      {},
      useStore,
    );
    expect(useStore.getState().combat.active).toBe(true);
    expect(useStore.getState().combat.tokens[0]?.name).toBe('Warrior');
  });

  it('end_combat clears combat state', () => {
    useStore.getState().combat.startCombat('enc-8', [makeToken()]);
    combatToolHandlers['end_combat']?.({}, {}, useStore);
    expect(useStore.getState().combat.active).toBe(false);
    expect(useStore.getState().combat.tokens).toHaveLength(0);
  });

  it('show_aoe_template adds an entry to aoeTemplates', () => {
    combatToolHandlers['show_aoe_template']?.(
      {
        shape: 'sphere',
        origin: { x: 60, y: 90 },
        direction: 0,
        size: 20,
        school: 'conjuration',
      },
      {},
      useStore,
    );
    expect(useStore.getState().combat.aoeTemplates).toHaveLength(1);
    expect(useStore.getState().combat.aoeTemplates[0]?.shape).toBe('sphere');
  });
});
