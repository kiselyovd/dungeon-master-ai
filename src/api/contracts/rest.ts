import * as v from 'valibot';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
export const RestIntentSchema = v.variant('kind', [
  v.object({ kind: v.literal('short'), hit_dice: v.pipe(integer, v.maxValue(20)) }),
  v.object({ kind: v.literal('long') }),
  v.object({ kind: v.literal('wait'), minutes: v.pipe(integer, v.minValue(1), v.maxValue(1440)) }),
]);
export type RestIntent = v.InferOutput<typeof RestIntentSchema>;
export const RestRequestSchema = v.object({
  session_id: uuid,
  character_id: uuid,
  expected_revision: integer,
  causation_id: uuid,
  intent: RestIntentSchema,
});
export type RestRequest = v.InferOutput<typeof RestRequestSchema>;
const slots = v.record(
  v.pipe(v.string(), v.regex(/^[1-9]$/)),
  v.pipe(
    v.object({ maximum: integer, remaining: integer }),
    v.check((stock) => stock.remaining <= stock.maximum),
  ),
);
export const RestStatusSchema = v.pipe(
  v.object({
    campaign_id: uuid,
    session_id: uuid,
    character_id: uuid,
    revision: integer,
    hp: integer,
    max_hp: v.pipe(integer, v.minValue(1), v.maxValue(10000)),
    hit_die: v.picklist([6, 8, 10, 12]),
    hit_dice_remaining: v.pipe(integer, v.maxValue(20)),
    slots,
    elapsed_seconds: integer,
    long_rest_ready_in_seconds: integer,
    safe: v.boolean(),
    combat_active: v.boolean(),
  }),
  v.check((status) => status.hp <= status.max_hp),
);
export type RestStatus = v.InferOutput<typeof RestStatusSchema>;
export const RestResultSchema = v.object({
  status: RestStatusSchema,
  outcome: v.object({
    hp: integer,
    elapsed_seconds: integer,
    duration_minutes: integer,
    resources: v.object({
      hit_dice_remaining: integer,
      last_long_rest_completed: v.nullable(integer),
      spell_slots: v.record(v.string(), integer),
    }),
    rolls: v.array(v.unknown()),
  }),
});
export type RestResult = v.InferOutput<typeof RestResultSchema>;
