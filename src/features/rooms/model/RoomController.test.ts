import * as v from 'valibot';
import { expect, it, vi } from 'vitest';
import wire from '../../../api/__fixtures__/roomUpdate.json';
import { RoomUpdateSchema } from '../../../api/contracts/room';
import type { RoomConnection } from '../../../api/rooms';
import { RoomController, type RoomPorts } from './RoomController';

function fixture() {
  return v.parse(RoomUpdateSchema, structuredClone(wire));
}
function connection(roomId = wire.state.room.room_id): RoomConnection {
  const member = wire.state.room.members.find((member) => member.role === 'player');
  if (!member) throw new Error('fixture member required');
  return {
    kind: 'guest',
    room: { ...fixture().state.room, room_id: roomId },
    member_id: member.member_id,
    address: '127.0.0.1:4123',
    join_id: crypto.randomUUID(),
  };
}
function ports(): RoomPorts {
  return {
    load: vi.fn(async () => fixture()),
    pending: vi.fn(async () => null),
    act: vi.fn(async () => {}),
    stream: vi.fn(() => new Promise<void>(() => {})),
  };
}
it('discards a late private snapshot after switching rooms', async () => {
  const transport = ports();
  let resolveOld!: (value: ReturnType<typeof fixture>) => void;
  const roomB = '44444444-4444-4444-8444-444444444444';
  const newer = fixture();
  newer.state.room.room_id = roomB;
  newer.state.title = 'New room';
  transport.load = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveOld = resolve;
        }),
    )
    .mockResolvedValue(newer);
  const controller = new RoomController(transport);
  const old = controller.activate(connection());
  await controller.activate(connection(roomB));
  resolveOld(fixture());
  await old;
  expect(controller.getSnapshot().connection?.room.room_id).toBe(roomB);
  expect(controller.getSnapshot().state?.title).toBe('New room');
  controller.leave();
});
it('does not rewind current state when an older response arrives', async () => {
  const transport = ports();
  const controller = new RoomController(transport);
  await controller.activate(connection());
  const options = vi.mocked(transport.stream).mock.calls[0]?.[0];
  if (!options) throw new Error('subscription required');
  const fresh = fixture();
  fresh.head += 2;
  fresh.cursor = fresh.head;
  fresh.state.revision += 2;
  fresh.state.title = 'Fresh';
  options.onEvent({ kind: 'update', update: fresh });
  options.onEvent({ kind: 'update', update: fixture() });
  expect(controller.getSnapshot().state?.title).toBe('Fresh');
  expect(controller.getSnapshot().cursor).toBe(fresh.cursor);
  controller.leave();
});
it('retains the canonical failed mutation and retries without a new nonce', async () => {
  const transport = ports();
  const action = {
    kind: 'rest' as const,
    request_id: 'same-nonce',
    expected_revision: 4,
    intent: { kind: 'wait' as const, minutes: 1 },
  };
  transport.act = vi
    .fn()
    .mockRejectedValueOnce({ code: 'native_room_transport_failed' })
    .mockResolvedValue(undefined);
  transport.pending = vi
    .fn()
    .mockResolvedValueOnce(null)
    .mockResolvedValueOnce(action)
    .mockResolvedValueOnce(null);
  const controller = new RoomController(transport);
  await controller.activate(connection());
  await controller.perform(action);
  expect(controller.getSnapshot().pending).toEqual(action);
  await controller.retryPending();
  expect(transport.act).toHaveBeenNthCalledWith(1, wire.state.room.room_id, action);
  expect(transport.act).toHaveBeenNthCalledWith(2, wire.state.room.room_id, action);
  expect(controller.getSnapshot().pending).toBeNull();
  controller.leave();
});
it('resumes its durable DM request and ignores cancelled late narration', async () => {
  const transport = ports();
  const pending = {
    kind: 'dm_turn' as const,
    request_id: crypto.randomUUID(),
    player_message: 'Осматриваюсь',
    resume: false,
  };
  transport.pending = vi.fn().mockResolvedValue(pending);
  transport.stream = vi.fn(async (options: Parameters<RoomPorts['stream']>[0]) => {
    if (options.operation.kind === 'subscribe') return await new Promise<void>(() => {});
    options.onEvent({ kind: 'agent', event: { type: 'text_delta', text: 'Река' } });
  });
  const controller = new RoomController(transport);
  await controller.activate(connection());
  await controller.retryPending();
  const options = vi
    .mocked(transport.stream)
    .mock.calls.find((call) => call[0].operation.kind === 'dm_turn')?.[0];
  expect(options?.operation).toEqual({ ...pending, resume: true });
  controller.leave();
  options?.onEvent({ kind: 'agent', event: { type: 'text_delta', text: 'LATE' } });
  expect(controller.getSnapshot().dmText).toBe('');
  expect(controller.getSnapshot().state).toBeNull();
});

it('aborts old DM callbacks on a real timeline reset but preserves normal snapshots', async () => {
  const transport = ports();
  const controller = new RoomController(transport);
  await controller.activate(connection());
  const sending = controller.sendDm('Осматриваюсь');
  const subscription = vi
    .mocked(transport.stream)
    .mock.calls.find((call) => call[0].operation.kind === 'subscribe')?.[0];
  const dm = vi
    .mocked(transport.stream)
    .mock.calls.find((call) => call[0].operation.kind === 'dm_turn')?.[0];
  if (!subscription || !dm) throw new Error('streams required');
  dm.onEvent({ kind: 'agent', event: { type: 'text_delta', text: 'Текущий ответ' } });
  const normal = fixture();
  normal.reset = true;
  subscription.onEvent({ kind: 'update', update: normal });
  expect(dm.signal?.aborted).toBe(false);
  expect(controller.getSnapshot().dmText).toBe('Текущий ответ');
  const restored = fixture();
  restored.head += 1;
  restored.cursor = restored.head;
  restored.state.revision += 1;
  restored.timeline_reset_cursor = restored.cursor;
  restored.events = []; // Reset event can be outside the retained 128-event window.
  subscription.onEvent({ kind: 'update', update: restored });
  expect(dm.signal?.aborted).toBe(true);
  expect(controller.getSnapshot().busy).toBe(false);
  dm.onEvent({ kind: 'agent', event: { type: 'text_delta', text: 'STALE BEFORE RESTORE' } });
  expect(controller.getSnapshot().dmText).toBe('');
  expect(controller.getSnapshot().errorCode).toBe('room_timeline_restored');
  controller.leave();
  void sending;
});
