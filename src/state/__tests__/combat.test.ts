import { beforeEach, describe, expect, it } from 'vitest';
import type { CombatProjectionDto, SnapshotCombat } from '../../features/combat/model/types';
import { useStore } from '../useStore';

function projection(revision: number, hp = 10): CombatProjectionDto {
  return {
    schema_version: 1,
    encounter_id: 'encounter-1',
    revision,
    snapshot: {
      active: true,
      round: 1,
      current_combatant: 'hero',
      initiative: [{ id: 'hero', roll: 18, dex_tiebreak: 2 }],
      combatants: [
        {
          id: 'hero',
          name: 'Hero',
          max_hp: 10,
          current_hp: hp,
          temp_hp: 0,
          ac: 15,
          speed_ft: 30,
          initiative_roll: 18,
          dex_mod: 2,
          conditions: [],
          budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
          is_dead: false,
          position: { x: 1, y: 2 },
        },
      ],
    },
    events: [],
  };
}

beforeEach(() => useStore.setState(useStore.getInitialState()));

describe('authoritative combat projection slice', () => {
  it('replaces the entire projection and derives render fields', () => {
    expect(useStore.getState().combat.replaceProjection(projection(0))).toBe(true);
    const combat = useStore.getState().combat;
    expect(combat.encounterId).toBe('encounter-1');
    expect(combat.revision).toBe(0);
    expect(combat.tokens[0]).toMatchObject({ id: 'hero', hp: 10, x: 1, y: 2, isActive: true });
    expect(combat.actionUsed).toBe(false);
  });

  it('ignores stale and duplicate revisions without mutating HP', () => {
    useStore.getState().combat.replaceProjection(projection(2, 7));
    expect(useStore.getState().combat.replaceProjection(projection(2, 1))).toBe(false);
    expect(useStore.getState().combat.replaceProjection(projection(1, 0))).toBe(false);
    expect(useStore.getState().combat.tokens[0]?.hp).toBe(7);
  });

  it('tracks pending commands without changing the projection', () => {
    useStore.getState().combat.replaceProjection(projection(0));
    useStore.getState().combat.markCommandPending('cmd-1', 'move');
    expect(useStore.getState().combat.pendingCommands).toEqual({ 'cmd-1': 'move' });
    expect(useStore.getState().combat.tokens[0]).toMatchObject({ x: 1, y: 2 });
    useStore.getState().combat.settleCommand('cmd-1');
    expect(useStore.getState().combat.pendingCommands).toEqual({});
  });

  it('maps a legacy save snapshot through the one restore seam', () => {
    const snapshot: SnapshotCombat = {
      active: true,
      encounter_id: 'saved',
      round: 3,
      current_turn_id: 'foe',
      initiative: ['foe'],
      tokens: [
        {
          id: 'foe',
          name: 'Orc',
          hp: 8,
          max_hp: 15,
          ac: 12,
          x: 4,
          y: 5,
          conditions: ['frightened'],
          resistances: [],
          immunities: [],
          vulnerabilities: [],
        },
      ],
    };
    useStore.getState().combat.hydrate(snapshot);
    expect(useStore.getState().combat).toMatchObject({
      encounterId: 'saved',
      revision: 0,
      round: 3,
      currentTurnId: 'foe',
    });
    expect(useStore.getState().combat.tokens[0]).toMatchObject({ hp: 8, x: 4, y: 5 });
  });

  it('keeps AoE previews as local presentation state', () => {
    useStore.getState().combat.addAoeTemplate({
      id: 'aoe',
      shape: 'sphere',
      originX: 0,
      originY: 0,
      sizeInFt: 20,
      school: 'evocation',
      rotateDeg: 0,
      expiresAt: Date.now() + 1000,
    });
    expect(useStore.getState().combat.aoeTemplates).toHaveLength(1);
  });
});
