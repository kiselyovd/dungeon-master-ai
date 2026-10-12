import * as v from 'valibot';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
const commandId = v.pipe(integer, v.minValue(1), v.maxValue(65535));
const authority = v.variant('kind', [
  v.strictObject({ kind: v.literal('solo'), restore_generation: integer }),
  v.strictObject({
    kind: v.literal('room'),
    room_id: uuid,
    member_id: uuid,
    epoch: v.pipe(integer, v.minValue(1)),
    reset_cursor: integer,
  }),
]);
const outcome = v.variant('kind', [
  v.strictObject({ kind: v.literal('performed'), check_success: v.nullable(v.boolean()) }),
  v.strictObject({
    kind: v.literal('refused'),
    code: v.pipe(v.string(), v.maxLength(128), v.regex(/^[a-z_]+$/)),
  }),
]);

export const AgentRecoverySchema = v.strictObject({
  schema_version: v.literal(1),
  scope: v.strictObject({
    request_id: uuid,
    campaign_id: uuid,
    session_id: uuid,
    actor_id: uuid,
    authority,
  }),
  model: v.nullable(v.pipe(v.string(), v.maxLength(512))),
  phase: v.picklist([
    'running',
    'progress_unknown',
    'actions_pending',
    'needs_input',
    'narration_pending',
    'delivery_pending',
  ]),
  allowed_action: v.picklist(['wait', 'resume', 'clarify']),
  progress_known: v.boolean(),
  commands: v.pipe(
    v.array(
      v.strictObject({
        command_id: commandId,
        state: v.picklist(['pending', 'prepared', 'finished', 'skipped', 'needs_resolution']),
      }),
    ),
    v.maxLength(32),
  ),
  confirmed_effects: v.pipe(
    v.array(
      v.strictObject({
        command_id: commandId,
        step: commandId,
        action: v.picklist(['adventure', 'combat', 'rest', 'dice']),
        outcome,
      }),
    ),
    v.maxLength(1024),
  ),
  clarification: v.nullable(v.strictObject({ id: uuid, command_id: commandId })),
});
export type AgentRecoveryProjection = v.InferOutput<typeof AgentRecoverySchema>;

export function parseAgentRecovery(value: unknown): AgentRecoveryProjection | null {
  if (value === null) return null;
  const parsed = v.safeParse(AgentRecoverySchema, value);
  if (!parsed.success) throw new Error('agent_recovery_invalid');
  const result = parsed.output;
  const ids = new Set(result.commands.map((entry) => entry.command_id));
  if (
    ids.size !== result.commands.length ||
    (result.phase === 'running') !== (result.allowed_action === 'wait') ||
    (result.phase === 'needs_input') !== (result.allowed_action === 'clarify') ||
    (result.clarification !== null && !ids.has(result.clarification.command_id)) ||
    (!result.progress_known &&
      (result.commands.length > 0 ||
        result.confirmed_effects.length > 0 ||
        result.clarification !== null)) ||
    result.confirmed_effects.some((entry) => !ids.has(entry.command_id))
  )
    throw new Error('agent_recovery_invalid');
  return result;
}
