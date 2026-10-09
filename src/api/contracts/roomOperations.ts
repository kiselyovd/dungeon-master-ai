import * as v from 'valibot';
import { AdventureActionSchema } from './adventure';
import { RecoveryIntentSchema } from './recovery';
import { RestIntentSchema } from './rest';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
const nonce = v.pipe(v.string(), v.minLength(1), v.maxLength(128));
export const HeroIntentSchema = v.variant('kind', [
  v.strictObject({ kind: v.literal('end_turn') }),
  v.strictObject({ kind: v.literal('retreat') }),
  v.strictObject({
    kind: v.literal('move'),
    x: v.pipe(v.number(), v.integer()),
    y: v.pipe(v.number(), v.integer()),
  }),
  v.strictObject({
    kind: v.literal('weapon'),
    target: uuid,
    weapon_id: v.string(),
    mode: v.optional(v.picklist(['default', 'melee', 'thrown', 'ranged'])),
    two_handed: v.optional(v.boolean()),
  }),
  v.strictObject({
    kind: v.literal('spell'),
    spell_id: v.string(),
    targets: v.array(uuid),
    slot_level: integer,
  }),
]);
const actionOptions = [
  v.strictObject({
    kind: v.literal('leave'),
    request_id: nonce,
    expected_epoch: v.pipe(integer, v.minValue(1)),
  }),
  v.strictObject({
    kind: v.literal('recovery'),
    request_id: nonce,
    expected_revision: integer,
    intent: RecoveryIntentSchema,
  }),
  v.strictObject({
    kind: v.literal('submit_character'),
    proposal_id: uuid,
    character_id: uuid,
    character: v.record(v.string(), v.unknown()),
  }),
  v.strictObject({
    kind: v.literal('approve_character'),
    proposal_id: uuid,
    expected_revision: integer,
    causation_id: nonce,
  }),
  v.strictObject({
    kind: v.literal('adventure'),
    request_id: nonce,
    expected_revision: integer,
    action: AdventureActionSchema,
  }),
  v.strictObject({
    kind: v.literal('rest'),
    request_id: nonce,
    expected_revision: integer,
    intent: RestIntentSchema,
  }),
  v.strictObject({
    kind: v.literal('combat'),
    encounter_id: uuid,
    request_id: nonce,
    expected_revision: integer,
    action: HeroIntentSchema,
  }),
] as const;
export const RoomActionSchema = v.variant('kind', actionOptions);
export const PendingRoomActionSchema = v.variant('kind', [
  ...actionOptions,
  v.strictObject({
    kind: v.literal('dm_turn'),
    request_id: uuid,
    player_message: v.pipe(v.string(), v.minLength(1), v.maxLength(16384)),
    resume: v.boolean(),
    awaiting_clarification: v.optional(uuid),
    clarification: v.optional(
      v.strictObject({ id: uuid, reply: v.pipe(v.string(), v.minLength(1), v.maxLength(4096)) }),
    ),
  }),
]);
export type RoomAction = v.InferOutput<typeof RoomActionSchema>;
export type PendingRoomAction = v.InferOutput<typeof PendingRoomActionSchema>;
