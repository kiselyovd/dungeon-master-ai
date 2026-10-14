import { describe, expect, it, vi } from 'vitest';
import {
  connectionNotification,
  NotificationController,
  type NotificationEligibility,
  type NotificationEvent,
  ownTurnNotification,
  readinessNotification,
} from './notifications';

const scopeId = '00000000-0000-4000-8000-000000000001',
  actor = '00000000-0000-4000-8000-000000000002',
  peer = '00000000-0000-4000-8000-000000000003',
  encounter = '00000000-0000-4000-8000-000000000004';
const turn = {
  scopeId,
  encounterId: encounter,
  round: 1,
  ownActorId: actor,
  currentActorId: actor,
  ownActorAlive: true,
  active: true,
  confirmed: true,
};
const event = ownTurnNotification(turn) as NotificationEvent;
const eligible: NotificationEligibility = {
  optedIn: true,
  permission: 'granted',
  focused: false,
  scopeCurrent: true,
};
describe('confirmed notification classification', () => {
  it('uses actor identities and canonical turn metadata, without private content', () => {
    expect(event).toEqual({
      kind: 'own_turn',
      scopeId,
      eventId: `${encounter}:1:${actor}`,
      target: 'chat',
    });
    for (const patch of [
      { ownActorId: peer },
      { ownActorId: null },
      { confirmed: false },
      { ownActorAlive: false },
      { active: false },
      { round: 0 },
      { round: 1.5 },
    ])
      expect(ownTurnNotification({ ...turn, ...patch })).toBeNull();
  });
  it('requires confirmed readiness IDs instead of creating one from a status poll', () => {
    expect(readinessNotification('runtime_ready', scopeId, null, true)).toBeNull();
    expect(readinessNotification('download_ready', scopeId, actor, false)).toBeNull();
    expect(readinessNotification('runtime_ready', scopeId, actor, true)?.eventId).toBe(actor);
  });
  it('requires an established connection and actionable failure, not routine starting', () => {
    expect(connectionNotification(scopeId, 1, 7, true, false)).toBeNull();
    expect(connectionNotification(scopeId, 1, 7, false, true)).toBeNull();
    expect(connectionNotification(scopeId, 1, 7, true, true)?.eventId).toBe('1:7');
  });
});
describe('notification delivery policy', () => {
  const stored = new Set<string>();
  function fixture() {
    stored.clear();
    const send = vi.fn(async () => {});
    const consume = vi.fn(async (key: string) => {
      if (stored.has(key)) return false;
      stored.add(key);
      return true;
    });
    return { controller: new NotificationController({ consume }, { send }), consume, send };
  }
  it('deduplicates concurrent replay and a restarted controller with the same ledger', async () => {
    const f = fixture();
    expect(
      await Promise.all([
        f.controller.observe(event, () => eligible),
        f.controller.observe(event, () => eligible),
      ]),
    ).toEqual(['send', 'duplicate']);
    const restarted = new NotificationController({ consume: f.consume }, { send: f.send });
    expect(await restarted.observe(event, () => eligible)).toBe('duplicate');
    expect(f.send).toHaveBeenCalledOnce();
  });
  it.each([
    { ...eligible, optedIn: false },
    { ...eligible, permission: 'denied' as const },
    { ...eligible, permission: 'unknown' as const },
    { ...eligible, focused: true },
    { ...eligible, scopeCurrent: false },
  ])('consumes suppressed events so they do not replay later (%#)', async (state) => {
    const f = fixture();
    expect(await f.controller.observe(event, () => state)).toBe('suppressed');
    expect(await f.controller.observe(event, () => eligible)).toBe('duplicate');
    expect(f.send).not.toHaveBeenCalled();
  });
  it('checks current focus after durable consumption and refuses a stale scope', async () => {
    let consumed: ((fresh: boolean) => void) | undefined;
    const send = vi.fn(async () => {});
    let state = eligible;
    const controller = new NotificationController(
      {
        consume: () =>
          new Promise((resolve) => {
            consumed = resolve;
          }),
      },
      { send },
    );
    const result = controller.observe(event, () => state);
    await Promise.resolve();
    state = { ...eligible, focused: true, scopeCurrent: false };
    consumed?.(true);
    expect(await result).toBe('suppressed');
    expect(send).not.toHaveBeenCalled();
  });
  it('keeps sending failures observable and does not block subsequent events', async () => {
    const f = fixture();
    f.send.mockRejectedValueOnce(Error('native_notification_failed'));
    await expect(f.controller.observe(event, () => eligible)).rejects.toThrow(
      'native_notification_failed',
    );
    expect(
      await f.controller.observe({ ...event, eventId: `${encounter}:2:${actor}` }, () => eligible),
    ).toBe('send');
  });
});
