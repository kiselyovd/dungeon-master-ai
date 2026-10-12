import { useTranslation } from 'react-i18next';
import { Button } from '../ui/Button';
import styles from './LocalModeModal.module.css';
import { LocalLlmTab } from './settings/LocalLlmTab';
export function LocalModeModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation('local_mode');
  const { t: tc } = useTranslation('common');
  if (!open) return null;
  return (
    <div
      className={styles.backdrop}
      role="dialog"
      aria-modal="true"
      aria-labelledby="local-mode-title"
    >
      <div className={styles.modal}>
        <h2 id="local-mode-title">{t('title')}</h2>
        <LocalLlmTab />
        <div className={styles.actions}>
          <Button onClick={onClose}>{tc('close')}</Button>
        </div>
      </div>
    </div>
  );
}
