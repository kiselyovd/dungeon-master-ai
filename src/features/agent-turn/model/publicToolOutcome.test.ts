import { expect, it } from 'vitest';
import { publicToolOutcome } from './publicToolOutcome';

it('uses engine result fields and never turns arbitrary provider text into narration', () => {
  expect(
    publicToolOutcome('roll_dice', { total: 18, private_note: 'fixture hidden text' }),
  ).toEqual({ key: 'outcome_dice', values: { total: 18 } });
  expect(publicToolOutcome('apply_damage', { new_hp: 0, effective_damage: 5 })).toEqual({
    key: 'outcome_hit_points',
    values: { hp: 0 },
  });
  expect(publicToolOutcome('apply_healing', { new_hp: 12 })).toEqual({
    key: 'outcome_hit_points',
    values: { hp: 12 },
  });
  for (const result of [
    { total: 'fixture hidden text' },
    { total: Number.NaN },
    { total: 1.5 },
    ['hidden'],
    null,
  ])
    expect(publicToolOutcome('roll_dice', result)).toEqual({ key: 'outcome_done', values: {} });
  expect(publicToolOutcome('unknown_tool', { total: 9, secret: 'fixture hidden text' })).toEqual({
    key: 'outcome_done',
    values: {},
  });
});
