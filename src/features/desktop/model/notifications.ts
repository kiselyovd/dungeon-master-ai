/** Only public, confirmed metadata enters the desktop notification boundary. */
export type NotificationKind =
  | 'own_turn'
  | 'runtime_ready'
  | 'download_ready'
  | 'connection_failed';
export interface NotificationEvent {
  kind: NotificationKind;
  /** Stable canonical scope, never a display name, path, URI or credential. */
  scopeId: string;
  eventId: string;
  target: 'chat' | 'downloads' | 'connection';
}
export interface TurnObservation {
  scopeId: string;
  encounterId: string;
  round: number;
  ownActorId: string | null;
  currentActorId: string | null;
  ownActorAlive: boolean;
  active: boolean;
  confirmed: boolean;
}
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
export function ownTurnNotification(turn: TurnObservation): NotificationEvent | null {
  if (
    !turn.confirmed ||
    !turn.active ||
    !turn.ownActorAlive ||
    !turn.ownActorId ||
    turn.ownActorId !== turn.currentActorId ||
    !UUID.test(turn.scopeId) ||
    !UUID.test(turn.encounterId) ||
    !UUID.test(turn.ownActorId) ||
    !Number.isSafeInteger(turn.round) ||
    turn.round < 1
  )
    return null;
  return {
    kind: 'own_turn',
    scopeId: turn.scopeId,
    eventId: `${turn.encounterId}:${turn.round}:${turn.ownActorId}`,
    target: 'chat',
  };
}
export function readinessNotification(
  kind: 'runtime_ready' | 'download_ready',
  scopeId: string,
  confirmedEventId: string | null,
  ready: boolean,
): NotificationEvent | null {
  if (!ready || !UUID.test(scopeId) || !confirmedEventId || !UUID.test(confirmedEventId))
    return null;
  return { kind, scopeId, eventId: confirmedEventId, target: 'downloads' };
}
export function connectionNotification(
  scopeId: string,
  epoch: number,
  cursor: number,
  actionable: boolean,
  hadConfirmedConnection: boolean,
): NotificationEvent | null {
  if (
    !actionable ||
    !hadConfirmedConnection ||
    !UUID.test(scopeId) ||
    !Number.isSafeInteger(epoch) ||
    epoch < 1 ||
    !Number.isSafeInteger(cursor) ||
    cursor < 0
  )
    return null;
  return {
    kind: 'connection_failed',
    scopeId,
    eventId: `${epoch}:${cursor}`,
    target: 'connection',
  };
}
export interface NotificationEligibility {
  optedIn: boolean;
  permission: 'granted' | 'denied' | 'unknown';
  focused: boolean;
  scopeCurrent: boolean;
}
export type NotificationDecision = 'send' | 'duplicate' | 'suppressed';
export interface NotificationLedger {
  /** Must durably consume an event once, including events observed in focus. */
  consume(key: string): Promise<boolean>;
}
export interface NotificationPort {
  send(event: NotificationEvent): Promise<void>;
}
export function notificationKey(event: NotificationEvent): string {
  return `${event.kind}:${event.scopeId}:${event.eventId}`;
}
/** Serial evaluation prevents replay/subscription races from sending twice. */
export class NotificationController {
  private lane: Promise<unknown> = Promise.resolve();
  constructor(
    private readonly ledger: NotificationLedger,
    private readonly port: NotificationPort,
  ) {}
  observe(
    event: NotificationEvent,
    current: () => NotificationEligibility,
  ): Promise<NotificationDecision> {
    const result = this.lane.then(async (): Promise<NotificationDecision> => {
      if (!(await this.ledger.consume(notificationKey(event)))) return 'duplicate';
      const eligibility = current();
      if (
        !eligibility.optedIn ||
        eligibility.permission !== 'granted' ||
        eligibility.focused ||
        !eligibility.scopeCurrent
      )
        return 'suppressed';
      await this.port.send(event);
      return 'send';
    });
    this.lane = result.catch(() => {});
    return result;
  }
}
