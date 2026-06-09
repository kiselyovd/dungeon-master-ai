import { beforeEach, describe, expect, it } from 'vitest';
import { chebyshevFt, DEFAULT_SPEED_FT } from '../combat';
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

  // W1.3 - movement budget consumption
  describe('tryMoveToken', () => {
    it('moves within budget: updates x/y and decrements movementRemaining by Chebyshev feet', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-move-1', [
        { id: 'pc', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      ]);
      // Move 2 cells diagonally = Chebyshev 2 = 10 ft
      const ok = useStore.getState().combat.tryMoveToken('pc', 2, 2);
      expect(ok).toBe(true);
      const state = useStore.getState().combat;
      const token = state.tokens.find((t) => t.id === 'pc');
      expect(token?.x).toBe(2);
      expect(token?.y).toBe(2);
      expect(state.movementRemaining).toBe(DEFAULT_SPEED_FT - 10);
    });

    it('rejects a move that exceeds movementRemaining: x/y and budget unchanged', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-move-2', [
        { id: 'pc', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      ]);
      // Move 7 cells = 35 ft, budget is 30
      const ok = useStore.getState().combat.tryMoveToken('pc', 7, 0);
      expect(ok).toBe(false);
      const state = useStore.getState().combat;
      const token = state.tokens.find((t) => t.id === 'pc');
      expect(token?.x).toBe(0);
      expect(token?.y).toBe(0);
      expect(state.movementRemaining).toBe(DEFAULT_SPEED_FT);
    });

    it('allows sequential moves that together exhaust but do not exceed budget', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-move-3', [
        { id: 'pc', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      ]);
      // First move: 3 cells = 15 ft
      useStore.getState().combat.tryMoveToken('pc', 3, 0);
      // Second move: 3 more cells = 15 ft (budget now 0)
      const ok = useStore.getState().combat.tryMoveToken('pc', 6, 0);
      expect(ok).toBe(true);
      expect(useStore.getState().combat.movementRemaining).toBe(0);
      // Third move: any further movement is rejected
      const rejected = useStore.getState().combat.tryMoveToken('pc', 7, 0);
      expect(rejected).toBe(false);
    });

    it('returns false for an unknown token id', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-move-4', [
        { id: 'pc', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      ]);
      const ok = useStore.getState().combat.tryMoveToken('nonexistent', 1, 1);
      expect(ok).toBe(false);
    });
  });

  // W1.3 - speed-aware econReset
  describe('speed-aware movement reset', () => {
    it('movementRemaining resets to active token speed on endTurn', () => {
      const { combat } = useStore.getState();
      // Foe has speed 20; when it becomes active the budget should be 20
      combat.startCombat('enc-speed-1', [
        {
          id: 'hero',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: [],
          speed: 30,
        },
        {
          id: 'foe',
          name: 'Goblin',
          hp: 7,
          maxHp: 7,
          ac: 13,
          x: 1,
          y: 0,
          conditions: [],
          speed: 20,
        },
      ]);
      // Initially hero's turn: budget = 30
      expect(useStore.getState().combat.movementRemaining).toBe(30);
      useStore.getState().combat.endTurn();
      // Foe's turn: budget = 20
      expect(useStore.getState().combat.movementRemaining).toBe(20);
    });

    it('startCombat sets movementRemaining to the first token speed', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-speed-2', [
        {
          id: 'hero',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: [],
          speed: 40,
        },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(40);
    });

    it('setCurrentTurn resets movementRemaining to the new active token speed', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-speed-3', [
        {
          id: 'a',
          name: 'Alpha',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: [],
          speed: 30,
        },
        { id: 'b', name: 'Beta', hp: 10, maxHp: 10, ac: 14, x: 1, y: 0, conditions: [], speed: 25 },
      ]);
      useStore.getState().combat.setCurrentTurn('b');
      expect(useStore.getState().combat.movementRemaining).toBe(25);
    });

    it('defaults to DEFAULT_SPEED_FT when token has no speed field', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-speed-4', [
        // No speed field - should default
        { id: 'hero', name: 'Hero', hp: 10, maxHp: 10, ac: 14, x: 0, y: 0, conditions: [] },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(DEFAULT_SPEED_FT);
    });
  });

  // W1.5 - condition-aware turn-start movement reset
  describe('condition-aware econReset', () => {
    it('restrained active token gets movementRemaining 0 on startCombat', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-1', [
        {
          id: 'pc',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: ['restrained'],
          speed: 30,
        },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(0);
    });

    it('grappled active token gets movementRemaining 0 on startCombat', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-2', [
        {
          id: 'pc',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: ['grappled'],
          speed: 30,
        },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(0);
    });

    it('stunned active token gets movementRemaining 0 on startCombat', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-3', [
        {
          id: 'pc',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: ['stunned'],
          speed: 30,
        },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(0);
    });

    it('non-restricting condition (poisoned) does not reduce movementRemaining', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-4', [
        {
          id: 'pc',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: ['poisoned'],
          speed: 30,
        },
      ]);
      expect(useStore.getState().combat.movementRemaining).toBe(30);
    });

    it('restrained active token gets movementRemaining 0 on setCurrentTurn', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-5', [
        {
          id: 'hero',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: [],
          speed: 30,
        },
        {
          id: 'foe',
          name: 'Goblin',
          hp: 7,
          maxHp: 7,
          ac: 13,
          x: 1,
          y: 0,
          conditions: ['restrained'],
          speed: 30,
        },
      ]);
      useStore.getState().combat.setCurrentTurn('foe');
      expect(useStore.getState().combat.movementRemaining).toBe(0);
    });

    it('restrained token gets movementRemaining 0 on endTurn (when it becomes active)', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-6', [
        {
          id: 'hero',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: [],
          speed: 30,
        },
        {
          id: 'foe',
          name: 'Goblin',
          hp: 7,
          maxHp: 7,
          ac: 13,
          x: 1,
          y: 0,
          conditions: ['restrained'],
          speed: 30,
        },
      ]);
      // endTurn: hero -> foe (foe is restrained)
      useStore.getState().combat.endTurn();
      expect(useStore.getState().combat.currentTurnId).toBe('foe');
      expect(useStore.getState().combat.movementRemaining).toBe(0);
    });

    it('restrained token: tryMoveToken returns false (movementRemaining is 0)', () => {
      const { combat } = useStore.getState();
      combat.startCombat('enc-cond-7', [
        {
          id: 'pc',
          name: 'Hero',
          hp: 10,
          maxHp: 10,
          ac: 14,
          x: 0,
          y: 0,
          conditions: ['restrained'],
          speed: 30,
        },
      ]);
      const ok = useStore.getState().combat.tryMoveToken('pc', 1, 0);
      expect(ok).toBe(false);
      expect(useStore.getState().combat.tokens[0]?.x).toBe(0);
    });
  });

  // chebyshevFt utility
  describe('chebyshevFt', () => {
    it('returns 0 for same cell', () => {
      expect(chebyshevFt(3, 3, 3, 3)).toBe(0);
    });

    it('returns 5 for one cell orthogonal', () => {
      expect(chebyshevFt(0, 0, 1, 0)).toBe(5);
    });

    it('returns 5 for one cell diagonal (Chebyshev = max of deltas)', () => {
      expect(chebyshevFt(0, 0, 1, 1)).toBe(5);
    });

    it('returns 30 for 6 cells orthogonal', () => {
      expect(chebyshevFt(0, 0, 6, 0)).toBe(30);
    });

    it('returns correct value for mixed deltas', () => {
      // dx=3, dy=5 -> max=5 -> 25 ft
      expect(chebyshevFt(0, 0, 3, 5)).toBe(25);
    });
  });
});
