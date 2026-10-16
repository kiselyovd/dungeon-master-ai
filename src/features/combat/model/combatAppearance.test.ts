import { expect, it } from 'vitest';
import wire from '../../../api/__fixtures__/soloCombatVitality.json';
import { parseCombatProjection, projectionTokens } from './combatProjection';

it('keeps public opponent appearance across renamed actors without guessing from names', () => {
  const value = structuredClone(wire.ended);
  const enemy = value.snapshot.combatants.find((actor) => !actor.character_id);
  if (!enemy) throw new Error('fixture opponent missing');
  enemy.name = 'An unrelated label';
  const parsed = parseCombatProjection({ ...value, appearances: { [enemy.id]: 'water_echo' } });
  if (!parsed) throw new Error('valid appearance rejected');
  expect(projectionTokens(parsed).find((actor) => actor.id === enemy.id)).toMatchObject({
    opponentAppearance: 'water_echo',
    x: enemy.position.x,
    y: enemy.position.y,
    hp: enemy.current_hp,
    ac: enemy.ac,
  });
  const legacy = parseCombatProjection(value);
  if (!legacy) throw new Error('legacy projection rejected');
  expect(projectionTokens(legacy).every((actor) => !actor.opponentAppearance)).toBe(true);
  expect(parseCombatProjection({ ...value, appearances: { hidden: 'water_echo' } })).toBeNull();
  const human = value.snapshot.combatants.find((actor) => actor.character_id);
  if (!human) throw new Error('fixture human missing');
  expect(parseCombatProjection({ ...value, appearances: { [human.id]: 'water_echo' } })).toBeNull();
  expect(
    parseCombatProjection({ ...value, appearances: { [enemy.id]: 'https://remote.invalid' } }),
  ).toBeNull();
  // A later client catalogue may add another bounded ID. This version renders
  // its neutral marker instead of rejecting the otherwise valid game state.
  expect(
    parseCombatProjection({ ...value, appearances: { [enemy.id]: 'future_opponent' } }),
  ).not.toBeNull();
});
