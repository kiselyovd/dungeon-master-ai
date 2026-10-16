import { invoke, isTauri } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import * as v from 'valibot';

const preferences = v.strictObject({
  enabled: v.boolean(),
  appScopeId: v.pipe(v.string(), v.uuid()),
  permission: v.picklist(['granted', 'denied', 'unknown', 'unsupported']),
});
const event = v.strictObject({
  kind: v.picklist(['own_turn', 'runtime_ready', 'download_ready', 'connection_failed']),
  scopeId: v.pipe(v.string(), v.uuid()),
  eventId: v.pipe(v.string(), v.maxLength(128)),
  target: v.picklist(['chat', 'downloads', 'connection']),
});
export type DesktopNotificationPreferences = v.InferOutput<typeof preferences>;
export type DesktopNotificationEvent = v.InferOutput<typeof event>;
export type NotificationOutcome = 'sent' | 'duplicate' | 'suppressed' | 'denied' | 'stale';
const browser: DesktopNotificationPreferences = {
  enabled: false,
  appScopeId: '00000000-0000-4000-8000-000000000001',
  permission: 'unsupported',
};
export async function notificationPreferences(): Promise<DesktopNotificationPreferences> {
  return isTauri()
    ? v.parse(preferences, await invoke('desktop_notification_preferences'))
    : browser;
}
export async function setNotifications(
  enabled: boolean,
  language: 'en' | 'ru',
): Promise<DesktopNotificationPreferences> {
  if (!isTauri()) throw Error('native_notification_unavailable');
  return v.parse(preferences, await invoke('desktop_set_notifications', { enabled, language }));
}
export async function setNotificationContext(
  gameScope: string | null,
  language: 'en' | 'ru',
): Promise<void> {
  if (isTauri()) await invoke('desktop_notification_context', { gameScope, language });
}
export async function offerNotification(
  value: DesktopNotificationEvent,
  language: 'en' | 'ru',
): Promise<NotificationOutcome> {
  if (!isTauri()) return 'suppressed';
  return v.parse(
    v.picklist(['sent', 'duplicate', 'suppressed', 'denied', 'stale']),
    await invoke('desktop_offer_notification', { event: value, language }),
  );
}
export async function onNotificationActivated(
  handler: (value: DesktopNotificationEvent) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const window = getCurrentWindow();
  if (window.label !== 'main') return () => {};
  return window.listen<unknown>('desktop-notification-activated', (message) => {
    const parsed = v.safeParse(event, message.payload);
    if (parsed.success) handler(parsed.output);
  });
}
