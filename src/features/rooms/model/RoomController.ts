import type { RoomState, RoomUpdate } from '../../../api/contracts/room';
import type { PendingRoomAction, RoomAction } from '../../../api/contracts/roomOperations';
import type { RoomStreamOptions } from '../../../api/roomStream';
import type { RoomConnection } from '../../../api/rooms';
export interface RoomPorts {
  load(roomId: string): Promise<RoomUpdate>;
  pending(roomId: string): Promise<PendingRoomAction | null>;
  act(roomId: string, action: RoomAction): Promise<void>;
  stream(options: RoomStreamOptions): Promise<void>;
}
export interface RoomView {
  connection: RoomConnection | null;
  state: RoomState | null;
  cursor: number | null;
  status: 'idle' | 'connecting' | 'connected' | 'disconnected';
  busy: boolean;
  errorCode: string | null;
  pending: PendingRoomAction | null;
  dmText: string;
}
function empty(): RoomView {
  return {
    connection: null,
    state: null,
    cursor: null,
    status: 'idle',
    busy: false,
    errorCode: null,
    pending: null,
    dmText: '',
  };
}
function errorCode(error: unknown): string {
  return typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string'
    ? error.code
    : 'native_room_request_failed';
}
/** Ephemeral presentation controller. Native and server own durable intentions and authority. */
export class RoomController {
  private view = empty();
  private generation = 0;
  private dmSequence = 0;
  private head = 0;
  private timelineReset = 0;
  private subscription: AbortController | null = null;
  private dm: AbortController | null = null;
  private listeners = new Set<() => void>();
  constructor(private readonly ports: RoomPorts) {}
  readonly getSnapshot = (): RoomView => this.view;
  readonly subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  private patch(patch: Partial<RoomView>) {
    this.view = { ...this.view, ...patch };
    for (const listener of this.listeners) listener();
  }
  private current(generation: number) {
    return this.generation === generation && this.view.connection !== null;
  }
  leave() {
    this.generation++;
    this.dmSequence++;
    this.subscription?.abort();
    this.dm?.abort();
    this.subscription = null;
    this.dm = null;
    this.head = 0;
    this.timelineReset = 0;
    this.view = empty();
    for (const listener of this.listeners) listener();
  }
  private apply(update: RoomUpdate, generation: number) {
    if (
      !this.current(generation) ||
      update.state.room.room_id !== this.view.connection?.room.room_id
    )
      return;
    if (
      update.head < this.head ||
      (this.view.state && update.state.revision < this.view.state.revision)
    )
      return;
    const reset = update.timeline_reset_cursor;
    const changedTimeline = reset > this.timelineReset;
    this.timelineReset = reset;
    const interruptedDm = changedTimeline && this.dm !== null;
    if (changedTimeline) {
      this.dmSequence++;
      this.dm?.abort();
      this.dm = null;
    }
    this.head = update.head;
    this.patch({
      state: update.state,
      cursor: Math.max(this.view.cursor ?? 0, update.cursor),
      ...(changedTimeline ? { dmText: '', errorCode: 'room_timeline_restored' } : {}),
      ...(interruptedDm ? { busy: false } : {}),
    });
  }
  async activate(connection: RoomConnection) {
    this.leave();
    const generation = this.generation;
    this.patch({ connection, status: 'connecting' });
    try {
      await this.refresh(generation);
      if (!this.current(generation)) return;
      this.patch({ status: 'connected', errorCode: null });
      this.listen(generation);
    } catch (error) {
      if (this.current(generation))
        this.patch({ status: 'disconnected', errorCode: errorCode(error) });
    }
  }
  private listen(generation: number) {
    const connection = this.view.connection;
    if (!connection) return;
    this.subscription?.abort();
    const controller = new AbortController();
    this.subscription = controller;
    void this.ports
      .stream({
        roomId: connection.room.room_id,
        operation: { kind: 'subscribe', after: this.view.cursor },
        signal: controller.signal,
        onEvent: (event) => {
          if (event.kind === 'update' && !controller.signal.aborted)
            this.apply(event.update, generation);
        },
      })
      .catch((error) => {
        if (this.current(generation) && !controller.signal.aborted)
          this.patch({ status: 'disconnected', errorCode: errorCode(error) });
      });
  }
  private async refresh(generation: number) {
    const connection = this.view.connection;
    if (!connection) return;
    const reset = this.timelineReset;
    const [update, pending] = await Promise.all([
      this.ports.load(connection.room.room_id),
      this.ports.pending(connection.room.room_id),
    ]);
    if (!this.current(generation) || reset !== this.timelineReset) return;
    this.apply(update, generation);
    this.patch({ pending });
  }
  async reconnect() {
    const connection = this.view.connection;
    if (connection) await this.activate(connection);
  }
  async perform(action: RoomAction) {
    if (this.view.status !== 'connected' || this.view.busy || !this.view.connection) return;
    if (this.view.pending && JSON.stringify(this.view.pending) !== JSON.stringify(action)) {
      this.patch({ errorCode: 'native_room_action_pending' });
      return;
    }
    const generation = this.generation;
    const roomId = this.view.connection.room.room_id;
    this.patch({ busy: true, pending: action, errorCode: null });
    try {
      await this.ports.act(roomId, action);
      if (this.current(generation)) await this.refresh(generation);
    } catch (error) {
      if (this.current(generation)) {
        this.patch({ errorCode: errorCode(error) });
        try {
          await this.refresh(generation);
        } catch {
          if (this.current(generation)) this.patch({ status: 'disconnected' });
        }
      }
    } finally {
      if (this.current(generation)) this.patch({ busy: false });
    }
  }
  async sendDm(text: string) {
    if (!text.trim() || !this.view.state?.own_character) return;
    if (this.view.pending) {
      this.patch({ errorCode: 'native_room_action_pending' });
      return;
    }
    if (this.view.state.agent_busy) {
      this.patch({ errorCode: 'native_room_turn_running' });
      return;
    }
    await this.runDm({
      kind: 'dm_turn',
      request_id: crypto.randomUUID(),
      player_message: text,
      resume: false,
    });
  }
  private async runDm(operation: Extract<PendingRoomAction, { kind: 'dm_turn' }>) {
    if (this.view.status !== 'connected' || this.view.busy || !this.view.connection) return;
    const generation = this.generation;
    const sequence = ++this.dmSequence;
    const roomId = this.view.connection.room.room_id;
    const controller = new AbortController();
    this.dm = controller;
    const current = () => this.current(generation) && this.dmSequence === sequence;
    this.patch({ busy: true, pending: operation, dmText: '', errorCode: null });
    try {
      await this.ports.stream({
        roomId,
        operation,
        signal: controller.signal,
        onEvent: (packet) => {
          if (!current() || controller.signal.aborted || packet.kind !== 'agent') return;
          if (packet.event.type === 'text_delta')
            this.patch({ dmText: this.view.dmText + packet.event.text });
        },
      });
      if (current()) {
        await this.refresh(generation);
        if (current()) this.patch({ dmText: '' });
      }
    } catch (error) {
      if (current()) {
        this.patch({
          errorCode: controller.signal.aborted ? 'native_room_turn_interrupted' : errorCode(error),
        });
        try {
          await this.refresh(generation);
        } catch {
          if (current()) this.patch({ status: 'disconnected' });
        }
      }
    } finally {
      if (current()) {
        this.dm = null;
        this.patch({ busy: false });
      }
    }
  }
  cancelDm() {
    this.dm?.abort();
  }
  async retryPending() {
    const pending = this.view.pending;
    if (!pending) return;
    if (pending.kind === 'dm_turn') await this.runDm({ ...pending, resume: true });
    else await this.perform(pending);
  }
}
