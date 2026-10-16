import { act, render, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { DesktopNotificationEvent } from '../../api/notifications';
import i18n from '../../i18n';
import {
  DesktopNotificationsProvider,
  useDesktopNotifications,
} from './DesktopNotificationsProvider';

const bridge = vi.hoisted(() => ({
  offer: vi.fn(async () => 'sent'),
  context: vi.fn(async () => {}),
  activate: undefined as undefined | ((event: DesktopNotificationEvent) => void),
  download: undefined as undefined | ((event: { kind: string; event_id?: string }) => void),
  cleanup: vi.fn(),
}));
vi.mock('../../api/notifications', () => ({
  offerNotification: bridge.offer,
  setNotificationContext: bridge.context,
  notificationPreferences: vi.fn(async () => ({
    enabled: true,
    appScopeId: '00000000-0000-4000-8000-000000000001',
    permission: 'granted',
  })),
  onNotificationActivated: vi.fn(async (handler: (event: DesktopNotificationEvent) => void) => {
    bridge.activate = handler;
    return bridge.cleanup;
  }),
}));
vi.mock('../../api/localLlm', () => ({
  subscribeDownloadEvents: vi.fn(
    async (handler: (event: { kind: string; event_id?: string }) => void) => {
      bridge.download = handler;
      return bridge.cleanup;
    },
  ),
}));
const scope = '00000000-0000-4000-8000-000000000002';
const event: DesktopNotificationEvent = {
  kind: 'own_turn',
  scopeId: scope,
  eventId: '00000000-0000-4000-8000-000000000003:1:00000000-0000-4000-8000-000000000004',
  target: 'chat',
};
function Target({
  scopeId = scope,
  onActivate = vi.fn(),
  events = [event],
}: {
  scopeId?: string;
  onActivate?: (event: DesktopNotificationEvent) => void;
  events?: DesktopNotificationEvent[];
}) {
  useDesktopNotifications({ scopeId, priority: 1, events, activate: onActivate });
  return null;
}
beforeEach(async () => {
  vi.clearAllMocks();
  bridge.activate = undefined;
  bridge.download = undefined;
  await i18n.changeLanguage('en');
});
it('sets native current scope before offering a canonical own-turn event', async () => {
  render(
    <DesktopNotificationsProvider>
      <Target />
    </DesktopNotificationsProvider>,
  );
  await waitFor(() => expect(bridge.offer).toHaveBeenCalled());
  expect(bridge.context).toHaveBeenCalledWith(scope, 'en');
  expect(bridge.context.mock.invocationCallOrder[0]).toBeLessThan(
    bridge.offer.mock.invocationCallOrder[0] ?? 0,
  );
});
it('updates the native runtime notification language when the UI language changes', async () => {
  render(
    <DesktopNotificationsProvider>
      <Target events={[]} />
    </DesktopNotificationsProvider>,
  );
  await waitFor(() => expect(bridge.context).toHaveBeenCalledWith(scope, 'en'));
  await act(async () => {
    await i18n.changeLanguage('ru');
  });
  await waitFor(() => expect(bridge.context).toHaveBeenCalledWith(scope, 'ru'));
});
it('refuses a stale click after a scope switch, while current clicks remain actionable', async () => {
  const clicked = vi.fn();
  const view = render(
    <DesktopNotificationsProvider>
      <Target onActivate={clicked} />
    </DesktopNotificationsProvider>,
  );
  await waitFor(() => expect(bridge.activate).toBeDefined());
  view.rerender(
    <DesktopNotificationsProvider>
      <Target scopeId="00000000-0000-4000-8000-000000000005" onActivate={clicked} events={[]} />
    </DesktopNotificationsProvider>,
  );
  act(() => bridge.activate?.(event));
  expect(clicked).not.toHaveBeenCalled();
  act(() => bridge.activate?.({ ...event, scopeId: '00000000-0000-4000-8000-000000000005' }));
  expect(clicked).toHaveBeenCalledOnce();
});
it('ignores progress/legacy completion and uses the server completion receipt', async () => {
  render(
    <DesktopNotificationsProvider>
      <Target events={[]} />
    </DesktopNotificationsProvider>,
  );
  await waitFor(() => expect(bridge.download).toBeDefined());
  act(() => {
    bridge.download?.({ kind: 'progress' });
    bridge.download?.({ kind: 'completed' });
  });
  expect(bridge.offer).not.toHaveBeenCalled();
  act(() =>
    bridge.download?.({ kind: 'completed', event_id: '00000000-0000-4000-8000-000000000006' }),
  );
  await waitFor(() =>
    expect(bridge.offer).toHaveBeenCalledWith(
      {
        kind: 'download_ready',
        scopeId: '00000000-0000-4000-8000-000000000001',
        eventId: '00000000-0000-4000-8000-000000000006',
        target: 'downloads',
      },
      'en',
    ),
  );
});
