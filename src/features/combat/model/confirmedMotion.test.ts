import { describe, expect, it } from 'vitest';
import { type CombatMotionFrame, confirmedCombatEffects } from './confirmedMotion';
import type { CombatToken } from './types';

const hero: CombatToken = {
  id: 'hero',
  characterId: 'sheet',
  name: 'Same name',
  hp: 12,
  maxHp: 12,
  ac: 18,
  x: 1,
  y: 3,
  conditions: [],
  movementRemaining: 30,
  actionAvailable: true,
};
const enemy: CombatToken = {
  id: 'enemy',
  name: 'Same name',
  hp: 6,
  maxHp: 6,
  ac: 11,
  x: 4,
  y: 3,
  conditions: [],
};
const frame = (overrides: Partial<CombatMotionFrame> = {}): CombatMotionFrame => ({
  scopeKey: 'solo:campaign:timeline-1:encounter',
  revision: 4,
  round: 1,
  active: true,
  ownActorId: 'hero',
  currentTurnId: 'enemy',
  tokens: [hero, enemy],
  ...overrides,
});

describe('confirmed gameplay presentation', () => {
  it('uses canonical damage events for a hit absorbed by temporary HP, never for immunity or mismatched results', () => {
    const hit = { target: 'hero', effective_amount: 4, new_hp: 12 };
    expect(
      confirmedCombatEffects(frame(), frame({ revision: 5, events: [{ damage: [hit] }] })).map(
        ({ kind, amount }) => ({ kind, amount }),
      ),
    ).toEqual([{ kind: 'damage', amount: 4 }]);
    for (const damage of [
      { ...hit, effective_amount: 0 },
      { ...hit, new_hp: 10 },
      { ...hit, target: 'unknown' },
      { ...hit, effective_amount: '4' },
    ])
      expect(
        confirmedCombatEffects(frame(), frame({ revision: 5, events: [{ damage: [damage] }] })),
      ).toEqual([]);
  });
  it('shows simultaneous confirmed damage/heal/movement/resource changes for distinct IDs', () => {
    const before = frame({ tokens: [{ ...hero, hp: 8 }, enemy] });
    const after = frame({
      revision: 5,
      tokens: [
        { ...hero, hp: 10, x: 2, movementRemaining: 25, actionAvailable: false },
        { ...enemy, hp: 2 },
      ],
    });
    const effects = confirmedCombatEffects(before, after);
    expect(effects.map(({ actorId, kind, amount }) => ({ actorId, kind, amount }))).toEqual([
      { actorId: 'hero', kind: 'heal', amount: 2 },
      { actorId: 'hero', kind: 'resource', amount: undefined },
      { actorId: 'hero', kind: 'move', amount: undefined },
      { actorId: 'enemy', kind: 'damage', amount: 4 },
    ]);
    expect(new Set(effects.map((effect) => effect.id)).size).toBe(4);
  });

  it('never celebrates pending, denied, stale, replayed or unseen revisions', () => {
    const before = frame(),
      changed = [{ ...hero, hp: 1, x: 4 }, enemy];
    for (const revision of [3, 4, 6, 4.5, Number.NaN])
      expect(confirmedCombatEffects(before, frame({ revision, tokens: changed }))).toEqual([]);
    const accepted = frame({ revision: 5, tokens: changed });
    expect(confirmedCombatEffects(before, accepted)).not.toEqual([]);
    expect(confirmedCombatEffects(accepted, accepted)).toEqual([]);
  });

  it('suppresses hydration, reconnect baseline and restored/changed scope', () => {
    const after = frame({ revision: 5, tokens: [{ ...hero, hp: 3 }, enemy] });
    expect(confirmedCombatEffects(null, after)).toEqual([]);
    expect(confirmedCombatEffects(frame(), { ...after, scopeKey: 'restored:timeline-2' })).toEqual(
      [],
    );
    expect(confirmedCombatEffects(frame(), { ...after, ownActorId: 'enemy' })).toEqual([]);
    expect(confirmedCombatEffects(frame({ active: false }), after)).toEqual([]);
    expect(confirmedCombatEffects(frame(), { ...after, active: false })).toEqual([]);
  });

  it('announces only the arrival of the living owned actor turn', () => {
    const after = frame({ revision: 5, currentTurnId: 'hero' });
    expect(confirmedCombatEffects(frame(), after).map((effect) => effect.kind)).toEqual([
      'own_turn',
    ]);
    expect(confirmedCombatEffects(frame({ currentTurnId: 'hero' }), after)).toEqual([]);
    expect(
      confirmedCombatEffects(frame({ currentTurnId: 'hero' }), frame({ revision: 5 })),
    ).toEqual([]);
    expect(
      confirmedCombatEffects(frame(), {
        ...after,
        tokens: [{ ...hero, hp: 0, isDead: true }],
      }).some((effect) => effect.kind === 'own_turn'),
    ).toBe(false);
  });

  it('recognizes a new own turn after server-automated actors return to the same hero', () => {
    const before = frame({ currentTurnId: 'hero' });
    const after = frame({ revision: 5, round: 2, currentTurnId: 'hero' });
    expect(confirmedCombatEffects(before, after).map((effect) => effect.kind)).toEqual([
      'own_turn',
    ]);
    expect(confirmedCombatEffects(before, { ...after, currentTurnId: 'enemy' })).toEqual([]);
  });

  it('does not compare newly joined or rebound actors by their shared display name', () => {
    const after = frame({
      revision: 5,
      tokens: [
        { ...hero, characterId: 'other-sheet', hp: 1 },
        { ...enemy, id: 'new-actor', hp: 1 },
      ],
    });
    expect(confirmedCombatEffects(frame(), after)).toEqual([]);
  });

  it('marks slot/ammunition consumption while replenishment stays a static update', () => {
    const caster = {
      ...hero,
      casting: {
        level: 1,
        ability_modifier: 2,
        attack_bonus: 4,
        save_dc: 12,
        prepared: [],
        choices: [],
        slots: { '1': { maximum: 2, remaining: 2 } },
      },
    };
    const spent = {
      ...caster,
      casting: { ...caster.casting, slots: { '1': { maximum: 2, remaining: 1 } } },
    };
    expect(
      confirmedCombatEffects(
        frame({ tokens: [caster] }),
        frame({ revision: 5, tokens: [spent] }),
      ).map((effect) => effect.kind),
    ).toEqual(['resource']);
    expect(
      confirmedCombatEffects(frame({ tokens: [spent] }), frame({ revision: 5, tokens: [caster] })),
    ).toEqual([]);
  });
});
