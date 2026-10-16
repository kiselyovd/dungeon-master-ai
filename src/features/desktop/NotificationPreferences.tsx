import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  type DesktopNotificationPreferences,
  notificationPreferences,
  setNotifications,
} from '../../api/notifications';
import { Field } from '../../ui/Field';
export function NotificationPreferences() {
  const { t, i18n } = useTranslation('common');
  const [state, setState] = useState<DesktopNotificationPreferences | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    void notificationPreferences()
      .then((value) => {
        if (active) setState(value);
      })
      .catch(() => {
        if (active) setFailed(true);
      });
    return () => {
      active = false;
    };
  }, []);
  return (
    <section aria-label={t('notifications_title')}>
      <Field label={t('notifications_enable')}>
        {(props) => (
          <input
            {...props}
            type="checkbox"
            checked={state?.enabled ?? false}
            disabled={busy || !state || state.permission === 'unsupported'}
            onChange={(event) => {
              const enabled = event.target.checked;
              setBusy(true);
              setFailed(false);
              void setNotifications(enabled, i18n.language.startsWith('ru') ? 'ru' : 'en')
                .then(setState)
                .catch(() => setFailed(true))
                .finally(() => setBusy(false));
            }}
          />
        )}
      </Field>
      <p>{t('notifications_hint')}</p>
      {state?.permission === 'denied' && <p role="status">{t('notifications_denied')}</p>}
      {state?.permission === 'unknown' && <p role="status">{t('notifications_unknown')}</p>}
      {state?.permission === 'unsupported' && <p role="status">{t('notifications_unsupported')}</p>}
      {failed && <p role="alert">{t('notifications_failed')}</p>}
    </section>
  );
}
