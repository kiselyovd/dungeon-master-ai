import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import styles from './ProviderMigrationBanner.module.css';

/**
 * One-time banner shown when a legacy `activeProvider:'anthropic'` was reset to
 * an unconfigured OpenAI-compatible cloud state during rehydration (M11 Batch
 * D.5). The `providerMigrationNotice` flag is transient (not persisted), so the
 * banner appears once per launch until dismissed.
 */
export function ProviderMigrationBanner() {
  const { t } = useTranslation('settings');
  const show = useStore((s) => s.settings.providerMigrationNotice);
  const dismiss = useStore((s) => s.settings.dismissProviderMigrationNotice);
  if (!show) return null;
  return (
    <div role="status" className={styles.banner}>
      <div className={styles.text}>
        <strong>{t('provider_migration_notice_title')}</strong>
        <p>{t('provider_migration_notice_body')}</p>
      </div>
      <button
        type="button"
        className={styles.dismiss}
        onClick={dismiss}
        aria-label={t('provider_migration_notice_dismiss')}
      >
        <Icons.X size={14} />
        {t('provider_migration_notice_dismiss')}
      </button>
    </div>
  );
}
