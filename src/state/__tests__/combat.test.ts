import { beforeEach, describe, expect, it } from 'vitest';
import { useStore } from '../useStore';

beforeEach(() => {
  useStore.setState((s) => ({
    ...s,
    combat: {
      ...s.combat,
      active: false,
      encounterId: null,
      tokens: [],
      initiativeOrder: [],
      currentTurnId: null,
      round: 1,
    },
  }));
});

describe('combat slice', () => {
  it('starts inactive', () => {
    const { combat } = useStore.getState();
    expect(combat.active).toBe(false);
    expect(combat.tokens).toHaveLength(0);
  });

  it('startCombat sets active and stores tokens', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-1', [
      { id: 'tok-1', name: 'Hero', hp: 15, maxHp: 15, ac: 14, x: 0, y: 0, conditions: [] },
      { id: 'tok-2', name: 'Goblin', hp: 7, maxHp: 7, ac: 13, x: 3, y: 2, conditions: [] },
    ]);
    const state = useStore.getState().combat;
    expect(state.active).toBe(true);
    expect(state.encounterId).toBe('enc-1');
    expect(state.tokens).toHaveLength(2);
  });

  it('applyDamage updates token hp', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-2', [
      { id: 'tok-a', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
    ]);
    combat.applyDamage('tok-a', 3);
    const token = useStore.getState().combat.tokens.find((t) => t.id === 'tok-a');
    expect(token?.hp).toBe(7);
  });

  it('endCombat clears state', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-3', [
      { id: 'tok-b', name: 'Goblin', hp: 7, maxHp: 7, ac: 13, x: 0, y: 0, conditions: [] },
    ]);
    combat.endCombat();
    expect(useStore.getState().combat.active).toBe(false);
    expect(useStore.getState().combat.tokens).toHaveLength(0);
  });

  it('addCondition and removeCondition mutate token conditions', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-4', [
      { id: 'tok-c', name: 'Wizard', hp: 8, maxHp: 8, ac: 11, x: 0, y: 0, conditions: [] },
    ]);
    combat.addCondition('tok-c', 'prone');
    expect(useStore.getState().combat.tokens[0]?.conditions).toContain('prone');
    combat.removeCondition('tok-c', 'prone');
    expect(useStore.getState().combat.tokens[0]?.conditions).not.toContain('prone');
  });
});
