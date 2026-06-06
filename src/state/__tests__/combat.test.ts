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
      aoeTemplates: [],
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

  it('useAction sets actionUsed to true', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-1', [
      { id: 't1', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
    ]);
    useStore.getState().combat.useAction();
    expect(useStore.getState().combat.actionUsed).toBe(true);
  });

  it('useBonus sets bonusUsed to true', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-2', [
      { id: 't1', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
    ]);
    useStore.getState().combat.useBonus();
    expect(useStore.getState().combat.bonusUsed).toBe(true);
  });

  it('useReaction sets reactionUsed to true', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-3', [
      { id: 't1', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
    ]);
    useStore.getState().combat.useReaction();
    expect(useStore.getState().combat.reactionUsed).toBe(true);
  });

  it('moveBy decrements movementRemaining, clamped at 0', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-4', [
      { id: 't1', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
    ]);
    useStore.getState().combat.moveBy(10);
    expect(useStore.getState().combat.movementRemaining).toBe(20);
    useStore.getState().combat.moveBy(25);
    expect(useStore.getState().combat.movementRemaining).toBe(0);
  });

  it('setCurrentTurn resets all action-economy fields', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-5', [
      { id: 't1', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      { id: 't2', name: 'Foe', hp: 8, maxHp: 8, ac: 13, x: 1, y: 1, conditions: [] },
    ]);
    useStore.getState().combat.useAction();
    useStore.getState().combat.useBonus();
    useStore.getState().combat.moveBy(30);
    useStore.getState().combat.setCurrentTurn('t2');
    const state = useStore.getState().combat;
    expect(state.actionUsed).toBe(false);
    expect(state.bonusUsed).toBe(false);
    expect(state.reactionUsed).toBe(false);
    expect(state.movementRemaining).toBe(30);
  });

  it('endTurn advances currentTurnId and resets economy', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-econ-6', [
      { id: 'x', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      { id: 'y', name: 'Foe', hp: 8, maxHp: 8, ac: 13, x: 1, y: 1, conditions: [] },
    ]);
    useStore.getState().combat.useAction();
    useStore.getState().combat.endTurn();
    const state = useStore.getState().combat;
    expect(state.currentTurnId).toBe('y');
    expect(state.actionUsed).toBe(false);
    expect(state.bonusUsed).toBe(false);
    expect(state.reactionUsed).toBe(false);
    expect(state.movementRemaining).toBe(30);
  });

  it('endTurn increments round only when the turn order wraps back to the top', () => {
    const { combat } = useStore.getState();
    combat.startCombat('enc-round-wrap', [
      { id: 'x', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      { id: 'y', name: 'Foe', hp: 8, maxHp: 8, ac: 13, x: 1, y: 1, conditions: [] },
    ]);
    expect(useStore.getState().combat.round).toBe(1);
    useStore.getState().combat.endTurn(); // x -> y, no wrap
    expect(useStore.getState().combat.round).toBe(1);
    useStore.getState().combat.endTurn(); // y -> x, wraps -> new round
    expect(useStore.getState().combat.currentTurnId).toBe('x');
    expect(useStore.getState().combat.round).toBe(2);
  });

  it('addAoeTemplate appends to aoeTemplates', () => {
    useStore.getState().combat.addAoeTemplate({
      id: 'aoe-1',
      shape: 'sphere',
      originX: 60,
      originY: 90,
      sizeInFt: 20,
      school: 'conjuration',
      rotateDeg: 0,
      expiresAt: Date.now() + 3000,
    });
    expect(useStore.getState().combat.aoeTemplates).toHaveLength(1);
  });

  it('removeAoeTemplate removes by id', () => {
    useStore.getState().combat.addAoeTemplate({
      id: 'aoe-r',
      shape: 'cone',
      originX: 0,
      originY: 0,
      sizeInFt: 15,
      school: 'evocation',
      rotateDeg: 0,
      expiresAt: Date.now() + 3000,
    });
    useStore.getState().combat.removeAoeTemplate('aoe-r');
    expect(useStore.getState().combat.aoeTemplates).toHaveLength(0);
  });
});
