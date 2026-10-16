import { beforeEach, expect, it, vi } from 'vitest';
import wireFixture from './__fixtures__/roomUpdate.json';
import { type RoomStreamEvent, streamRoom } from './roomStream';

const mocked = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mocked.invoke,
  Channel: class {
    onmessage: (value: unknown) => void = () => {};
  },
}));
const roomId = '11111111-1111-4111-8111-111111111111';
const requestId = '22222222-2222-4222-8222-222222222222';
const operation = {
  kind: 'dm_turn' as const,
  request_id: requestId,
  player_message: 'Слушаю',
  resume: false,
};
beforeEach(() => mocked.invoke.mockReset());
function scripts(frames: string[], foreign = false, scopeRoom = roomId) {
  mocked.invoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    if (command !== 'stream_room') return;
    const channel = args.onChunk as { onmessage: (value: unknown) => void };
    const send = (bytes: number[], room = scopeRoom) =>
      channel.onmessage({ room_id: room, stream_id: args.streamId, bytes });
    if (foreign)
      send(
        Array.from(new TextEncoder().encode('event: text_delta\ndata: {"text":"FOREIGN"}\n\n')),
        '33333333-3333-4333-8333-333333333333',
      );
    for (const frame of frames) for (const byte of new TextEncoder().encode(frame)) send([byte]);
  });
}
it('isolates channel scope, decodes split Unicode and excludes private reasoning', async () => {
  scripts(
    [
      'event: reasoning_text\ndata: {"text":"PRIVATE"}\n\n',
      'event: text_delta\ndata: {"text":"Река"}\n\n',
      'event: agent_done\ndata: {"total_rounds":1}\n\n',
    ],
    true,
  );
  const events: RoomStreamEvent[] = [];
  await streamRoom({ roomId, operation, onEvent: (event) => events.push(event) });
  expect(events).toEqual([
    { kind: 'agent', event: { type: 'text_delta', text: 'Река' } },
    { kind: 'agent', event: { type: 'agent_done', totalRounds: 1 } },
  ]);
  const args = mocked.invoke.mock.calls.find((call) => call[0] === 'stream_room')?.[1];
  expect(args).not.toHaveProperty('capability');
  expect(args).not.toHaveProperty('address');
  expect(mocked.invoke).toHaveBeenCalledWith('cancel_room_stream', { streamId: args.streamId });
});
it('does not treat an EOF or legacy done as confirmed DM completion', async () => {
  scripts(['event: done\ndata: {}\n\n']);
  await expect(streamRoom({ roomId, operation, onEvent: () => {} })).rejects.toMatchObject({
    code: 'native_room_stream_interrupted',
  });
});
it('does not open a native stream after an early abort', async () => {
  const controller = new AbortController();
  controller.abort();
  await expect(
    streamRoom({ roomId, operation, signal: controller.signal, onEvent: () => {} }),
  ).rejects.toMatchObject({ code: 'native_room_stream_cancelled' });
  expect(mocked.invoke).not.toHaveBeenCalled();
});
it('preserves a recovery code from native and requests cancellation on failure', async () => {
  mocked.invoke.mockImplementation(async (command) => {
    if (command === 'stream_room') throw 'native_room_turn_interrupted';
  });
  await expect(streamRoom({ roomId, operation, onEvent: () => {} })).rejects.toMatchObject({
    code: 'native_room_turn_interrupted',
  });
  expect(mocked.invoke.mock.calls.some((call) => call[0] === 'cancel_room_stream')).toBe(true);
});

it('rejects a mismatched room snapshot and cancels its stream', async () => {
  scripts(['event: room_update\ndata: {"cursor":1}\n\n']);
  await expect(
    streamRoom({ roomId, operation: { kind: 'subscribe', after: null }, onEvent: () => {} }),
  ).rejects.toMatchObject({ code: 'native_room_stream_invalid' });
  expect(mocked.invoke.mock.calls.some((call) => call[0] === 'cancel_room_stream')).toBe(true);
});
it('cancels an in-flight invocation after abort and ignores its later packets', async () => {
  const controller = new AbortController();
  const events: RoomStreamEvent[] = [];
  mocked.invoke.mockImplementation(async (command, args) => {
    if (command !== 'stream_room') return;
    controller.abort();
    args.onChunk.onmessage({
      room_id: roomId,
      stream_id: args.streamId,
      bytes: Array.from(
        new TextEncoder().encode('event: agent_done\ndata: {"total_rounds":1}\n\n'),
      ),
    });
  });
  await expect(
    streamRoom({
      roomId,
      operation,
      signal: controller.signal,
      onEvent: (event) => events.push(event),
    }),
  ).rejects.toMatchObject({ code: 'native_room_stream_cancelled' });
  expect(events).toEqual([]);
  expect(mocked.invoke.mock.calls.some((call) => call[0] === 'cancel_room_stream')).toBe(true);
});

it('does not accept completion after a DM error event', async () => {
  scripts([
    'event: error\ndata: {"code":"provider_error","message":"failed"}\n\n',
    'event: agent_done\ndata: {"total_rounds":1}\n\n',
  ]);
  await expect(streamRoom({ roomId, operation, onEvent: () => {} })).rejects.toMatchObject({
    code: 'native_room_dm_failed',
  });
});

it('delivers a snapshot serialized by the real Rust room stream', async () => {
  const actualRoom = wireFixture.state.room.room_id;
  scripts([`event: room_update\ndata: ${JSON.stringify(wireFixture)}\n\n`], false, actualRoom);
  const events: RoomStreamEvent[] = [];
  await expect(
    streamRoom({
      roomId: actualRoom,
      operation: { kind: 'subscribe', after: null },
      onEvent: (event) => events.push(event),
    }),
  ).rejects.toMatchObject({ code: 'native_room_stream_interrupted' });
  expect(events).toHaveLength(1);
  expect(events[0]).toMatchObject({
    kind: 'update',
    update: {
      state: {
        own_character: { name: 'Tara' },
        campaign: { created_at_epoch_ms: expect.any(Number) },
      },
    },
  });
});
