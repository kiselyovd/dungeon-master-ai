import { describe, expect, it } from 'vitest';
import { aggregateConditionEffects, getConditionEffect } from '../conditions';

describe('aggregateConditionEffects', () => {
  it('returns defaults (no restriction) for an empty list', () => {
    const result = aggregateConditionEffects([]);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('unknown condition names have no effect', () => {
    const result = aggregateConditionEffects(['cursed', 'hexed', 'FOOBAR']);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('stunned -> preventsActions, preventsReactions, movementMultiplier 0', () => {
    const result = aggregateConditionEffects(['stunned']);
    expect(result.preventsActions).toBe(true);
    expect(result.preventsReactions).toBe(true);
    expect(result.movementMultiplier).toBe(0);
  });

  it('paralyzed -> preventsActions, preventsReactions, movementMultiplier 0', () => {
    const result = aggregateConditionEffects(['paralyzed']);
    expect(result.preventsActions).toBe(true);
    expect(result.preventsReactions).toBe(true);
    expect(result.movementMultiplier).toBe(0);
  });

  it('unconscious -> preventsActions, preventsReactions, movementMultiplier 0', () => {
    const result = aggregateConditionEffects(['unconscious']);
    expect(result.preventsActions).toBe(true);
    expect(result.preventsReactions).toBe(true);
    expect(result.movementMultiplier).toBe(0);
  });

  it('petrified -> preventsActions, preventsReactions, movementMultiplier 0', () => {
    const result = aggregateConditionEffects(['petrified']);
    expect(result.preventsActions).toBe(true);
    expect(result.preventsReactions).toBe(true);
    expect(result.movementMultiplier).toBe(0);
  });

  it('incapacitated -> preventsActions, preventsReactions, movementMultiplier 0', () => {
    const result = aggregateConditionEffects(['incapacitated']);
    expect(result.preventsActions).toBe(true);
    expect(result.preventsReactions).toBe(true);
    expect(result.movementMultiplier).toBe(0);
  });

  it('restrained -> movementMultiplier 0, actions allowed', () => {
    const result = aggregateConditionEffects(['restrained']);
    expect(result.movementMultiplier).toBe(0);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('grappled -> movementMultiplier 0, actions allowed', () => {
    const result = aggregateConditionEffects(['grappled']);
    expect(result.movementMultiplier).toBe(0);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('prone -> no movement/action block', () => {
    const result = aggregateConditionEffects(['prone']);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('poisoned -> no movement/action block', () => {
    const result = aggregateConditionEffects(['poisoned']);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('frightened -> no movement/action block', () => {
    const result = aggregateConditionEffects(['frightened']);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  it('blinded -> no movement/action block', () => {
    const result = aggregateConditionEffects(['blinded']);
    expect(result.movementMultiplier).toBe(1);
    expect(result.preventsActions).toBe(false);
    expect(result.preventsReactions).toBe(false);
  });

  describe('case-insensitive matching', () => {
    it('STUNNED (uppercase) is treated same as stunned', () => {
      const result = aggregateConditionEffects(['STUNNED']);
      expect(result.preventsActions).toBe(true);
      expect(result.movementMultiplier).toBe(0);
    });

    it('Restrained (mixed case) is treated same as restrained', () => {
      const result = aggregateConditionEffects(['Restrained']);
      expect(result.movementMultiplier).toBe(0);
      expect(result.preventsActions).toBe(false);
    });
  });

  describe('combination takes the strictest effect', () => {
    it('restrained + stunned -> preventsActions true (OR), movementMultiplier 0 (min)', () => {
      const result = aggregateConditionEffects(['restrained', 'stunned']);
      expect(result.movementMultiplier).toBe(0);
      expect(result.preventsActions).toBe(true);
      expect(result.preventsReactions).toBe(true);
    });

    it('prone + restrained -> movementMultiplier 0 (restrained wins), no action block', () => {
      const result = aggregateConditionEffects(['prone', 'restrained']);
      expect(result.movementMultiplier).toBe(0);
      expect(result.preventsActions).toBe(false);
    });

    it('poisoned + frightened + unknown -> no hard blocks', () => {
      const result = aggregateConditionEffects(['poisoned', 'frightened', 'hexed']);
      expect(result.movementMultiplier).toBe(1);
      expect(result.preventsActions).toBe(false);
      expect(result.preventsReactions).toBe(false);
    });

    it('unconscious + grappled -> preventsActions true, movementMultiplier 0', () => {
      const result = aggregateConditionEffects(['unconscious', 'grappled']);
      expect(result.movementMultiplier).toBe(0);
      expect(result.preventsActions).toBe(true);
      expect(result.preventsReactions).toBe(true);
    });
  });
});

describe('getConditionEffect', () => {
  it('returns defaults for unknown condition', () => {
    const effect = getConditionEffect('vampiric_touch');
    expect(effect.movementMultiplier).toBe(1);
    expect(effect.preventsActions).toBe(false);
    expect(effect.preventsReactions).toBe(false);
  });

  it('returns correct effect for paralyzed', () => {
    const effect = getConditionEffect('paralyzed');
    expect(effect.preventsActions).toBe(true);
    expect(effect.preventsReactions).toBe(true);
    expect(effect.movementMultiplier).toBe(0);
  });
});
