import * as v from 'valibot';
import { PublicHeroRecoverySchema, RecoveryReceiptSchema } from './recovery';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
export const SoloRecoveryIntentSchema = v.variant('kind', [
  v.strictObject({ kind: v.literal('wait'), target_character_id: uuid }),
  v.strictObject({ kind: v.literal('death_save') }),
]);
export type SoloRecoveryIntent = v.InferOutput<typeof SoloRecoveryIntentSchema>;
export const SoloRecoveryRequestSchema = v.pipe(
  v.strictObject({
    session_id: uuid,
    character_id: uuid,
    expected_revision: integer,
    causation_id: uuid,
    intent: SoloRecoveryIntentSchema,
  }),
  v.check((r) => r.intent.kind !== 'wait' || r.intent.target_character_id === r.character_id),
);
export type SoloRecoveryRequest = v.InferOutput<typeof SoloRecoveryRequestSchema>;
export const SoloRecoveryStatusSchema = v.pipe(
  v.strictObject({
    campaign_id: uuid,
    session_id: uuid,
    character_id: uuid,
    revision: integer,
    hp: integer,
    max_hp: v.pipe(integer, v.minValue(1), v.maxValue(10000)),
    elapsed_seconds: integer,
    safe: v.boolean(),
    combat_active: v.boolean(),
    recovery: PublicHeroRecoverySchema,
  }),
  v.check((s) => {
    const h = s.recovery;
    if (s.hp > s.max_hp || h.character_id !== s.character_id) return false;
    if (s.hp > 0 !== (h.status === 'healthy')) return false;
    if (h.status === 'stable' && (h.successes !== 0 || h.failures !== 0)) return false;
    if (h.ready_at_seconds !== null && h.status !== 'stable') return false;
    return h.actions.every(
      (a) =>
        !s.combat_active &&
        s.hp === 0 &&
        (a.kind === 'wait'
          ? s.safe && h.status === 'stable' && a.target_character_id === s.character_id
          : a.kind === 'death_save' && h.status === 'dying'),
    );
  }),
);
export type SoloRecoveryStatus = v.InferOutput<typeof SoloRecoveryStatusSchema>;
export const SoloRecoveryResultSchema = v.pipe(
  v.strictObject({ status: SoloRecoveryStatusSchema, receipt: RecoveryReceiptSchema }),
  v.check(
    (r) =>
      r.receipt.receipt_revision === r.status.revision &&
      r.receipt.target_character_id === r.status.character_id &&
      r.receipt.medicine === null &&
      r.receipt.recovered === r.status.hp > 0 &&
      r.receipt.stabilized === (r.status.recovery.status === 'stable') &&
      r.receipt.ready_at_seconds === r.status.recovery.ready_at_seconds &&
      r.receipt.death_saves.every((d) => d.character_id === r.status.character_id),
  ),
);
export type SoloRecoveryResult = v.InferOutput<typeof SoloRecoveryResultSchema>;
