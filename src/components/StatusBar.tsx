import { useTranslation } from 'react-i18next';
import { Icons } from '../ui/Icons';
import styles from './StatusBar.module.css';

export type ProviderStatus = 'connected' | 'error' | 'loading';

interface Props {
  provider: string;
  model: string;
  status?: ProviderStatus;
  savedAgo?: { minutes: number } | 'now' | null;
}

export function StatusBar({ provider, model, status = 'connected', savedAgo = null }: Props) {
  const { t } = useTranslation('common');

  const dotClass =
    status === 'connected'
      ? styles.dotConnected
      : status === 'error'
        ? styles.dotError
        : styles.dotLoading;

  const savedLabel =
    savedAgo === null
      ? null
      : savedAgo === 'now'
        ? t('saved_now')
        : t('saved_min_ago', { count: savedAgo.minutes });

  return (
    <footer className={styles.statusBar} role="status" aria-live="polite">
      <div className={styles.item}>
        <span className={`${styles.dot} ${dotClass}`} />
        <span className={styles.label}>{t('provider')}:</span>
        <span className={styles.value}>{provider}</span>
      </div>
      <div className={styles.divider} />
      <div className={styles.item}>
        <Icons.Cpu size={11} />
        <span className={styles.label}>{t('model')}:</span>
        <span className={styles.valueMono}>{model}</span>
      </div>
      <div className={styles.spacer} />
      {savedLabel !== null && (
        <div className={styles.item}>
          <Icons.Save size={11} />
          <span className={styles.label}>{t('saved')}:</span>
          <span className={styles.value}>{savedLabel}</span>
        </div>
      )}
    </footer>
  );
}
