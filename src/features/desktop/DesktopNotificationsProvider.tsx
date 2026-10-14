import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
} from 'react';
import { useTranslation } from 'react-i18next';
import { subscribeDownloadEvents } from '../../api/localLlm';
import {
  type DesktopNotificationEvent,
  notificationPreferences,
  offerNotification,
  onNotificationActivated,
  setNotificationContext,
} from '../../api/notifications';
import { type NotificationEvent, readinessNotification } from './model/notifications';

interface Registration {
  scopeId: string | null;
  priority: number;
  events: NotificationEvent[];
  activate(event: DesktopNotificationEvent): void;
}
interface Registry {
  register(id: symbol, read: () => Registration): () => void;
  changed(): void;
}
const Context = createContext<Registry | null>(null);
export function DesktopNotificationsProvider({ children }: { children: ReactNode }) {
  const { i18n } = useTranslation();
  const language = useRef<'en' | 'ru'>(i18n.language.startsWith('ru') ? 'ru' : 'en');
  language.current = i18n.language.startsWith('ru') ? 'ru' : 'en';
  const currentLanguage = language.current;
  const registrations = useRef(new Map<symbol, () => Registration>());
  const [revision, changed] = useReducer((value) => value + 1, 0);
  const registry = useMemo<Registry>(
    () => ({
      register(id, read) {
        registrations.current.set(id, read);
        changed();
        return () => {
          registrations.current.delete(id);
          changed();
        };
      },
      changed,
    }),
    [],
  );
  const latest = useCallback(
    () =>
      [...registrations.current.values()]
        .map((read) => read())
        .sort((a, b) => b.priority - a.priority)[0],
    [],
  );
  const lane = useRef<Promise<unknown>>(Promise.resolve());
  const submit = useCallback((event: NotificationEvent) => {
    lane.current = lane.current
      .then(() => offerNotification(event, language.current))
      .catch(() => {
        console.warn('[desktop.notifications]', { code: 'native_notification_failed' });
      });
  }, []);
  // biome-ignore lint/correctness/useExhaustiveDependencies: registered scope/event updates explicitly trigger reconciliation.
  useEffect(() => {
    const current = latest();
    lane.current = lane.current
      .then(() => setNotificationContext(current?.scopeId ?? null, currentLanguage))
      .then(() => {
        for (const event of current?.events ?? []) submit(event);
      })
      .catch(() => {
        console.warn('[desktop.notifications]', { code: 'native_notification_context_failed' });
      });
  }, [revision, latest, submit, currentLanguage]);
  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void onNotificationActivated((event) => {
      const target = latest();
      if (target?.scopeId === event.scopeId || event.target === 'downloads')
        target?.activate(event);
    })
      .then((unlisten) => {
        if (active) cleanup = unlisten;
        else unlisten();
      })
      .catch(() => {
        console.warn('[desktop.notifications]', {
          code: 'native_notification_activation_unavailable',
        });
      });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [latest]);
  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void notificationPreferences()
      .then(async (preferences) => {
        if (!active || preferences.permission === 'unsupported') return;
        const unlisten = await subscribeDownloadEvents((event) => {
          const ready = readinessNotification(
            'download_ready',
            preferences.appScopeId,
            event.event_id ?? null,
            event.kind === 'completed',
          );
          if (active && ready) submit(ready);
        });
        if (active) cleanup = unlisten;
        else unlisten();
      })
      .catch(() => {
        console.warn('[desktop.notifications]', {
          code: 'native_download_notifications_unavailable',
        });
      });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [submit]);
  return <Context.Provider value={registry}>{children}</Context.Provider>;
}
export function useDesktopNotifications(registration: Registration): void {
  const context = useContext(Context);
  const id = useRef(Symbol('notifications'));
  const latest = useRef(registration);
  latest.current = registration;
  useLayoutEffect(() => context?.register(id.current, () => latest.current), [context]);
  const signature = JSON.stringify([registration.scopeId, registration.events]);
  useLayoutEffect(() => {
    if (signature) context?.changed();
  }, [context, signature]);
}
