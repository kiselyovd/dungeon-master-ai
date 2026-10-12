import { beforeEach, expect, it, vi } from 'vitest';
import aidWire from './__fixtures__/roomRecoveryAidUpdate.json';
import wire from './__fixtures__/roomUpdate.json';
import {
  joinRoom,
  loadPendingRoomAction,
  loadRoom,
  loadRoomAgentRecovery,
  recoverGuestRoom,
  recoverHostRoom,
  sendRoomAction,
} from './rooms';

const mocked = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocked.invoke }));
beforeEach(() => mocked.invoke.mockReset());
it('reads recovery through native without accepting another member projection', async () => {
  const roomId = wire.state.room.room_id;
  const memberId = '11111111-1111-4111-8111-111111111111';
  const recovery = {
    schema_version: 1,
    scope: {
      request_id: '22222222-2222-4222-8222-222222222222',
      campaign_id: wire.state.room.campaign_id,
      session_id: wire.state.room.session_id,
      actor_id: '33333333-3333-4333-8333-333333333333',
      authority: { kind: 'room', room_id: roomId, member_id: memberId, epoch: 1, reset_cursor: 0 },
    },
    model: null,
    phase: 'progress_unknown',
    allowed_action: 'resume',
    progress_known: false,
    commands: [],
    confirmed_effects: [],
    clarification: null,
  };
  mocked.invoke.mockResolvedValueOnce({ status: 200, body: recovery });
  expect(await loadRoomAgentRecovery(roomId, memberId)).toEqual(recovery);
  expect(mocked.invoke).toHaveBeenCalledWith('room_operation', {
    roomId,
    operation: { kind: 'agent_recovery' },
  });
  mocked.invoke.mockResolvedValueOnce({ status: 200, body: recovery });
  await expect(
    loadRoomAgentRecovery(roomId, '44444444-4444-4444-8444-444444444444'),
  ).rejects.toMatchObject({ code: 'native_room_response_invalid' });
});
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

it('recovers the selected host without renderer credentials and rejects a guest identity', async () => {
  const member = wire.state.room.members.find((member) => member.role === 'host');
  if (!member) throw new Error('host required');
  const view = {
    room: wire.state.room,
    member_id: member.member_id,
    address: '127.0.0.1:4123',
    invitation: 'fixture invitation',
  };
  mocked.invoke.mockResolvedValueOnce(view);
  expect(await recoverHostRoom()).toMatchObject({ kind: 'host', member_id: member.member_id });
  expect(mocked.invoke).toHaveBeenCalledWith('recover_host_room', {});
  mocked.invoke.mockResolvedValueOnce(null);
  expect(await recoverHostRoom()).toBeNull();
  mocked.invoke.mockResolvedValueOnce({
    ...view,
    member_id: wire.state.room.members.find((member) => member.role === 'player')?.member_id,
  });
  await expect(recoverHostRoom()).rejects.toMatchObject({ code: 'native_room_response_invalid' });
  mocked.invoke.mockResolvedValueOnce({ ...view, capability: 'must never enter renderer' });
  await expect(recoverHostRoom()).rejects.toMatchObject({ code: 'native_room_response_invalid' });
});

it('decodes a real recovery receipt and rejects client-supplied dice', async () => {
  const event = [...aidWire.events].reverse().find((event) => 'receipt' in event.payload);
  if (!event || !('receipt' in event.payload)) throw new Error('receipt required');
  mocked.invoke.mockResolvedValue({ status: 200, body: event.payload.receipt });
  const target = aidWire.state.recovery.find((hero) => hero.status === 'stable');
  if (!target) throw new Error('target required');
  const action = {
    kind: 'recovery' as const,
    request_id: crypto.randomUUID(),
    expected_revision: aidWire.state.revision - 1,
    intent: { kind: 'aid' as const, target_character_id: target.character_id },
  };
  await expect(sendRoomAction(aidWire.state.room.room_id, action)).resolves.toBeUndefined();
  mocked.invoke.mockClear();
  await expect(
    sendRoomAction(aidWire.state.room.room_id, {
      ...action,
      intent: { kind: 'death_save', roll: 20 },
    } as never),
  ).rejects.toMatchObject({ code: 'native_room_operation_invalid' });
  expect(mocked.invoke).not.toHaveBeenCalled();
});
