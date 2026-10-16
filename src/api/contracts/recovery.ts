import * as v from 'valibot';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
export const RecoveryIntentSchema = v.variant('kind', [
  v.strictObject({ kind: v.literal('aid'), target_character_id: uuid }),
  v.strictObject({ kind: v.literal('wait'), target_character_id: uuid }),
  v.strictObject({ kind: v.literal('death_save') }),
]);
export const PublicHeroRecoverySchema = v.strictObject({
  character_id: uuid,
  status: v.picklist(['healthy', 'dying', 'stable', 'dead']),
  successes: v.pipe(integer, v.maxValue(3)),
  failures: v.pipe(integer, v.maxValue(3)),
  ready_at_seconds: v.nullable(integer),
  actions: v.array(RecoveryIntentSchema),
});
const deathSave = v.strictObject({
  combatant: uuid,
  roll: v.pipe(integer, v.minValue(1), v.maxValue(20)),
  success: v.boolean(),
  successes_total: v.pipe(integer, v.maxValue(3)),
  failures_total: v.pipe(integer, v.maxValue(3)),
  dead: v.boolean(),
  stable: v.boolean(),
});
export const RecoveryReceiptSchema = v.strictObject({
  receipt_revision: integer,
  duplicate: v.boolean(),
  target_character_id: uuid,
  stabilized: v.boolean(),
  recovered: v.boolean(),
  ready_at_seconds: v.nullable(integer),
  medicine: v.nullable(
    v.strictObject({
      rolls: v.pipe(
        v.array(v.pipe(integer, v.minValue(1), v.maxValue(20))),
        v.minLength(1),
        v.maxLength(2),
      ),
      modifier: v.pipe(v.number(), v.integer()),
      total: v.pipe(v.number(), v.integer()),
      disadvantage: v.boolean(),
    }),
  ),
  death_saves: v.array(v.strictObject({ character_id: uuid, result: deathSave })),
});
