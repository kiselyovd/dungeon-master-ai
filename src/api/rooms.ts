import { invoke } from '@tauri-apps/api/core';
import * as v from 'valibot';
import { ProposalSummarySchema, PublicRoomSchema, RoomUpdateSchema } from './contracts/room';
import {
  PendingRoomActionSchema,
  type RoomAction,
  RoomActionSchema,
} from './contracts/roomOperations';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
const hostView = v.strictObject({
  room: PublicRoomSchema,
  member_id: uuid,
  address: v.string(),
  invitation: v.string(),
});
const guestView = v.strictObject({
  room: PublicRoomSchema,
  member_id: uuid,
  address: v.string(),
  join_id: uuid,
});
export type RoomConnection =
  | ({ kind: 'host' } & v.InferOutput<typeof hostView>)
  | ({ kind: 'guest' } & v.InferOutput<typeof guestView>);
export interface HostRoomRequest {
  request_id: string;
  campaign_id: string;
  session_id: string;
  expected_revision: number;
  bind_addr: string;
}
export interface JoinRoomRequest {
  address: string;
  room_id: string;
  join_id: string;
  display_name: string;
  invitation: string;
}
export class RoomClientError extends Error {
  constructor(
    readonly code: string,
    readonly status = 0,
    options?: ErrorOptions,
  ) {
    super(code, options);
    this.name = 'RoomClientError';
  }
}
function invalid(cause?: unknown): never {
  throw new RoomClientError('native_room_response_invalid', 0, { cause });
}
async function native(command: string, args: Record<string, unknown>): Promise<unknown> {
  try {
    return await invoke(command, args);
  } catch (cause) {
    throw new RoomClientError(
      typeof cause === 'string' && /^native_[a-z_]+$/.test(cause)
        ? cause
        : 'native_room_request_failed',
      0,
      { cause },
    );
  }
}
export async function hostRoom(request: HostRoomRequest): Promise<RoomConnection> {
  const parsed = v.safeParse(hostView, await native('host_room', { request }));
  if (!parsed.success) return invalid();
  const view = parsed.output;
  if (
    view.room.campaign_id !== request.campaign_id ||
    view.room.session_id !== request.session_id ||
    !view.room.active ||
    !view.room.members.some(
      (member) => member.member_id === view.member_id && member.active && member.role === 'host',
    )
  )
    return invalid();
  return { kind: 'host', ...view };
}
export async function joinRoom(request: JoinRoomRequest): Promise<RoomConnection> {
  const parsed = v.safeParse(guestView, await native('join_room', { request }));
  if (!parsed.success) return invalid();
  const view = parsed.output;
  if (
    view.room.room_id !== request.room_id.toLowerCase() ||
    view.join_id !== request.join_id.toLowerCase() ||
    !view.room.active ||
    !view.room.members.some(
      (member) => member.member_id === view.member_id && member.active && member.role === 'player',
    )
  )
    return invalid();
  return { kind: 'guest', ...view };
}
export async function recoverGuestRoom(): Promise<RoomConnection | null> {
  const raw = await native('recover_guest_room', {});
  if (raw === null) return null;
  const parsed = v.safeParse(guestView, raw);
  if (!parsed.success) return invalid();
  const view = parsed.output;
  if (
    !view.room.active ||
    !view.room.members.some(
      (member) => member.member_id === view.member_id && member.active && member.role === 'player',
    )
  )
    return invalid();
  return { kind: 'guest', ...view };
}
export async function stopHostRoom(roomId: string): Promise<void> {
  await native('stop_host_room', { roomId });
}
const resultSchema = v.strictObject({
  status: v.pipe(integer, v.minValue(100), v.maxValue(599)),
  body: v.nullable(v.unknown()),
});
async function execute(roomId: string, operation: unknown): Promise<unknown> {
  const result = v.safeParse(resultSchema, await native('room_operation', { roomId, operation }));
  if (!result.success) return invalid();
  if (result.output.status < 200 || result.output.status >= 300) {
    const body = result.output.body as { error?: { code?: unknown } } | null;
    const code =
      typeof body?.error?.code === 'string' && /^[a-z_]+$/.test(body.error.code)
        ? body.error.code
        : 'native_room_request_rejected';
    throw new RoomClientError(code, result.output.status);
  }
  return result.output.body;
}
export async function loadRoom(roomId: string) {
  const parsed = v.safeParse(RoomUpdateSchema, await execute(roomId, { kind: 'state' }));
  if (!parsed.success || parsed.output.state.room.room_id !== roomId) return invalid();
  return parsed.output;
}
export async function loadPendingRoomAction(roomId: string) {
  const body = await execute(roomId, { kind: 'pending_action' });
  if (body === null) return null;
  const parsed = v.safeParse(PendingRoomActionSchema, body);
  if (!parsed.success) return invalid();
  return parsed.output;
}
const worldReceipt = v.strictObject({
  receipt_revision: integer,
  duplicate: v.boolean(),
  rest_outcome: v.nullable(v.unknown()),
});
const combatReceipt = v.strictObject({
  projection: v.unknown(),
  duplicate: v.boolean(),
  receipt_revision: integer,
  receipt_events: v.array(v.unknown()),
});
export async function sendRoomAction(roomId: string, action: RoomAction): Promise<void> {
  const input = v.safeParse(RoomActionSchema, action);
  if (!input.success) throw new RoomClientError('native_room_operation_invalid');
  const body = await execute(roomId, input.output);
  const schema =
    action.kind === 'submit_character'
      ? ProposalSummarySchema
      : action.kind === 'approve_character'
        ? v.strictObject({ proposal: ProposalSummarySchema, revision: integer })
        : action.kind === 'combat'
          ? combatReceipt
          : worldReceipt;
  if (!v.safeParse(schema, body).success) return invalid();
}

const recoveryChoiceSchema = v.strictObject({
  room_id: uuid,
  join_id: uuid,
  address: v.string(),
  display_name: v.string(),
});
export type GuestRecoveryChoice = v.InferOutput<typeof recoveryChoiceSchema>;
export async function savedGuestRooms() {
  const parsed = v.safeParse(
    v.pipe(v.array(recoveryChoiceSchema), v.maxLength(64)),
    await native('saved_guest_rooms', {}),
  );
  if (!parsed.success) return invalid();
  return parsed.output;
}
export async function postponeGuestRoom() {
  await native('postpone_guest_room', {});
}
export async function resumeGuestRoom(choice: GuestRecoveryChoice): Promise<RoomConnection> {
  const parsed = v.safeParse(
    guestView,
    await native('resume_room', { request: { room_id: choice.room_id, join_id: choice.join_id } }),
  );
  if (!parsed.success) return invalid();
  const view = parsed.output;
  if (
    view.room.room_id !== choice.room_id ||
    view.join_id !== choice.join_id ||
    !view.room.active ||
    !view.room.members.some(
      (member) => member.member_id === view.member_id && member.active && member.role === 'player',
    )
  )
    return invalid();
  return { kind: 'guest', ...view };
}
