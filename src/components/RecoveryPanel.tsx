import { useTranslation } from 'react-i18next';
import type { RecoveryView } from '../features/recovery/model/recovery';
import { Button } from '../ui/Button';
import styles from './RecoveryPanel.module.css';

interface Props {
  view: RecoveryView | null;
  status: 'loading' | 'ready' | 'error';
  busy?: boolean;
  onResume: () => void;
  onRetry: () => void;
  onConfigure?: () => void;
}
export function RecoveryPanel({ view, status, busy, onResume, onRetry, onConfigure }: Props) {
  const { t } = useTranslation('chat');
  if (status === 'ready' && !view) return null;
  const label =
    status === 'error'
      ? 'recovery_check_failed'
      : status === 'loading'
        ? 'recovery_checking'
        : `recovery_${view?.phase}`;
  return (
    <section
      className={styles.panel}
      role={status === 'error' ? 'alert' : 'status'}
      data-testid="agent-recovery-panel"
    >
      <p>{t(label as 'recovery_checking')}</p>
      {view?.model && <p className={styles.detail}>{t('recovery_model', { model: view.model })}</p>}
      {view?.progressKnown && (
        <p className={styles.detail}>
          {t('recovery_progress', { done: view.finishedCommands, total: view.totalCommands })}
        </p>
      )}
      <div className={styles.actions}>
        {status === 'ready' && view?.action === 'resume' && (
          <Button disabled={busy} onClick={onResume}>
            {t('recovery_continue')}
          </Button>
        )}
        {status === 'error' && (
          <Button disabled={busy} onClick={onRetry}>
            {t('recovery_check_retry')}
          </Button>
        )}
        {onConfigure && status !== 'loading' && (
          <Button disabled={busy} onClick={onConfigure}>
            {t('recovery_settings')}
          </Button>
        )}
      </div>
    </section>
  );
}
