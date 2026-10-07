import { Channel, invoke } from '@tauri-apps/api/core';
import * as v from 'valibot';
import { type AgentEvent, decodeAgentEvent } from './contracts/agent';
import { type RoomStreamOperation, type RoomUpdate, RoomUpdateSchema } from './contracts/room';
import type { SseEvent } from './sse';
import { SseStreamDecoder } from './sseStream';

const uuid = v.pipe(v.string(), v.uuid());
const ChunkSchema = v.strictObject({
  room_id: uuid,
  stream_id: uuid,
  bytes: v.pipe(
    v.array(v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255))),
    v.maxLength(32768),
  ),
});
export type RoomStreamEvent =
  | { kind: 'update'; update: RoomUpdate }
  | { kind: 'agent'; event: AgentEvent };
export class RoomStreamError extends Error {
  constructor(
    readonly code: string,
    options?: ErrorOptions,
  ) {
    super(code, options);
    this.name = 'RoomStreamError';
  }
}
export interface RoomStreamOptions {
  roomId: string;
  operation: RoomStreamOperation;
  signal?: AbortSignal;
  onEvent: (event: RoomStreamEvent) => void;
}
/** Native owns credentials and destinations. Every invocation has an isolated channel. */
export async function streamRoom(options: RoomStreamOptions): Promise<void> {
  if (options.signal?.aborted) throw new RoomStreamError('native_room_stream_cancelled');
  const streamId = crypto.randomUUID();
  const channel = new Channel<unknown>();
  const decoder = new SseStreamDecoder(16 * 1024 * 1024);
  let terminal = false;
  let callbackFailure: unknown;
  const cancel = () => {
    void invoke('cancel_room_stream', { streamId }).catch(() => {});
  };
  const dispatch = (frames: SseEvent[]) => {
    for (const raw of frames) {
      if (options.signal?.aborted) return;
      if (raw.event === 'reasoning_text') continue;
      if (options.operation.kind === 'subscribe') {
        if (raw.event !== 'room_update') throw new RoomStreamError('native_room_stream_invalid');
        const update = v.parse(RoomUpdateSchema, raw.data);
        if (update.state.room.room_id !== options.roomId)
          throw new RoomStreamError('native_room_stream_invalid');
        options.onEvent({ kind: 'update', update });
      } else {
        const event = decodeAgentEvent(raw);
        if (!event || event.type === 'reasoning_text')
          throw new RoomStreamError('native_room_stream_invalid');
        if (event.type === 'agent_done') terminal = true;
        options.onEvent({ kind: 'agent', event });
        if (event.type === 'error') throw new RoomStreamError('native_room_dm_failed');
      }
    }
  };
  channel.onmessage = (raw) => {
    if (options.signal?.aborted || callbackFailure) return;
    try {
      const chunk = v.parse(ChunkSchema, raw);
      if (chunk.room_id !== options.roomId || chunk.stream_id !== streamId) return;
      dispatch(decoder.push(Uint8Array.from(chunk.bytes)));
    } catch (error) {
      callbackFailure =
        error instanceof RoomStreamError
          ? error
          : new RoomStreamError('native_room_stream_invalid', { cause: error });
      cancel();
    }
  };
  options.signal?.addEventListener('abort', cancel, { once: true });
  try {
    await invoke('stream_room', {
      roomId: options.roomId,
      streamId,
      operation: options.operation,
      onChunk: channel,
    });
    if (callbackFailure) throw callbackFailure;
    if (options.signal?.aborted) throw new RoomStreamError('native_room_stream_cancelled');
    dispatch(decoder.finish());
    if (!terminal) throw new RoomStreamError('native_room_stream_interrupted');
  } catch (error) {
    if (callbackFailure) throw callbackFailure;
    if (options.signal?.aborted)
      throw new RoomStreamError('native_room_stream_cancelled', { cause: error });
    if (error instanceof RoomStreamError) throw error;
    const code =
      typeof error === 'string' && /^native_room_[a-z_]+$/.test(error)
        ? error
        : 'native_room_stream_failed';
    throw new RoomStreamError(code, { cause: error });
  } finally {
    options.signal?.removeEventListener('abort', cancel);
    channel.onmessage = () => {};
    cancel();
  }
}
