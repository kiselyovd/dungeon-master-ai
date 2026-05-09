import { useTranslation } from 'react-i18next';
import styles from './UpdateAvailableModal.module.css';

interface Props {
  version: string;
  notes: string;
  onUpdate: () => void;
  onLater: () => void;
}

export function UpdateAvailableModal({ version, notes, onUpdate, onLater }: Props) {
  const { t } = useTranslation('updater');
  return (
    <div className={styles.backdrop} role="dialog" aria-labelledby="update-title" aria-modal="true">
      <div className={styles.modal}>
        <h2 id="update-title">{t('title', { version })}</h2>
        {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-label on <pre> conveys release-notes context to screen readers */}
        <pre className={styles.notes} aria-label={t('release_notes')}>
          {notes || '-'}
        </pre>
        <div className={styles.actions}>
          <button type="button" onClick={onLater}>
            {t('later')}
          </button>
          <button type="button" onClick={onUpdate} className={styles.primary}>
            {t('update_now')}
          </button>
        </div>
      </div>
    </div>
  );
}
