import { beforeEach, expect, it, vi } from 'vitest';
import wire from './__fixtures__/roomUpdate.json';
import {
  joinRoom,
  loadPendingRoomAction,
  loadRoom,
  recoverGuestRoom,
  sendRoomAction,
} from './rooms';

const mocked = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocked.invoke }));
beforeEach(() => mocked.invoke.mockReset());
it('decodes the current room using the real serialized wire snapshot', async () => {
  mocked.invoke.mockImplementation(async (command) =>
    command === 'room_operation' ? { status: 200, body: wire } : undefined,
  );
  expect((await loadRoom(wire.state.room.room_id)).state.own_character?.name).toBe('Tara');
  expect(mocked.invoke).toHaveBeenCalledWith('room_operation', {
    roomId: wire.state.room.room_id,
    operation: { kind: 'state' },
  });
});
it('preserves native pending conflicts without sending a different identity', async () => {
  mocked.invoke.mockImplementation(async (command) =>
    command === 'room_operation'
      ? { status: 409, body: { error: { code: 'native_room_action_pending' } } }
      : undefined,
  );
  await expect(loadPendingRoomAction(wire.state.room.room_id)).rejects.toMatchObject({
    status: 409,
    code: 'native_room_action_pending',
  });
});
it('rejects an actor override before invoking native transport', async () => {
  const action = {
    kind: 'rest',
    request_id: 'wait',
    expected_revision: 4,
    intent: { kind: 'wait', minutes: 1 },
    member_id: 'override',
  };
  await expect(sendRoomAction(wire.state.room.room_id, action as never)).rejects.toMatchObject({
    code: 'native_room_operation_invalid',
  });
  expect(mocked.invoke).not.toHaveBeenCalledWith('room_operation', expect.anything());
});
it('accepts canonical native address/UUID serialization and excludes credentials', async () => {
  const member = wire.state.room.members.find((member) => member.role === 'player');
  if (!member) throw new Error('guest required');
  const joinId = crypto.randomUUID();
  const view = {
    room: wire.state.room,
    member_id: member.member_id,
    join_id: joinId,
    address: '[::1]:4123',
  };
  mocked.invoke.mockImplementation(async (command) => (command === 'join_room' ? view : undefined));
  const request = {
    room_id: wire.state.room.room_id.toUpperCase(),
    join_id: joinId.toUpperCase(),
    address: '[0:0:0:0:0:0:0:1]:4123',
    display_name: 'Tara',
    invitation: 'test invitation',
  };
  expect((await joinRoom(request)).address).toBe('[::1]:4123');
  mocked.invoke.mockImplementation(async (command) =>
    command === 'join_room' ? { ...view, capability: 'forbidden' } : undefined,
  );
  await expect(joinRoom(request)).rejects.toMatchObject({ code: 'native_room_response_invalid' });
});

it('restores native guest selection without exposing credentials or requiring renderer IDs', async () => {
  const member = wire.state.room.members.find((member) => member.role === 'player');
  if (!member) throw new Error('guest required');
  const view = {
    room: wire.state.room,
    member_id: member.member_id,
    join_id: crypto.randomUUID(),
    address: '127.0.0.1:4123',
  };
  mocked.invoke.mockImplementation(async (command) =>
    command === 'recover_guest_room' ? view : undefined,
  );
  expect((await recoverGuestRoom())?.member_id).toBe(member.member_id);
  expect(mocked.invoke).toHaveBeenCalledWith('recover_guest_room', {});
  mocked.invoke.mockImplementation(async (command) =>
    command === 'recover_guest_room' ? null : undefined,
  );
  expect(await recoverGuestRoom()).toBeNull();
});
