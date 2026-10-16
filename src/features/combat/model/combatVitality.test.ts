import { expect, it } from 'vitest';
import wire from '../../../api/__fixtures__/soloCombatVitality.json';
import { parseCombatProjection, projectionTokens } from './combatProjection';
import { isDeadToken } from './selectors';

it('actual Rust dying actor mortality survives presentation mapping', () => {
  const projection = parseCombatProjection(wire.ended);
  if (!projection) throw new Error('invalid real fixture');
  const hero = projectionTokens(projection).find((t) => t.characterId === wire.before.character_id);
  if (!hero) throw new Error('missing human');
  expect(hero.hp).toBe(0);
  expect(hero.isDead).toBe(false);
  expect(isDeadToken(hero)).toBe(false);
});

it('rejects malformed withdrawal flags while allowing legacy absence', () => {
  const legacy = structuredClone(wire.ended);
  expect(parseCombatProjection(legacy)).not.toBeNull();
  const malformed = structuredClone(legacy);
  (malformed.snapshot.combatants[0] as unknown as Record<string, unknown>).withdrawn = 'false';
  expect(parseCombatProjection(malformed)).toBeNull();
});
