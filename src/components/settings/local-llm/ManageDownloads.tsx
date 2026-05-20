import { useTranslation } from 'react-i18next';
import type { MergedEntry } from '../../../state/local_llm/manifest';
import styles from './ManageDownloads.module.css';

export interface ManageDownloadsProps {
  models: MergedEntry[];
  onDownload: (id: string) => void;
  onDelete: (id: string) => void;
}

export function ManageDownloads({ models, onDownload, onDelete }: ManageDownloadsProps) {
  const { t } = useTranslation('local_llm');

  return (
    <ul className={styles.list}>
      {models.map((m) => {
        const pct = m.downloadProgress !== undefined ? Math.round(m.downloadProgress * 100) : null;
        const isDownloading =
          m.downloadState === 'queued' ||
          m.downloadState === 'downloading' ||
          m.downloadState === 'verifying';
        return (
          <li key={m.id} data-testid="download-row" className={styles.row}>
            <div className={styles.rowContent}>
              <span className={styles.label}>
                {m.display_name}{' '}
                <small>
                  ({m.size_gb} GB, {m.license})
                </small>
              </span>
              {!m.installed && !isDownloading && (
                <button type="button" onClick={() => onDownload(m.id)}>
                  {t('download')}
                </button>
              )}
              {m.installed && !isDownloading && (
                <button type="button" onClick={() => onDelete(m.id)}>
                  {t('delete')}
                </button>
              )}
              {isDownloading && (
                <button type="button" onClick={() => onDelete(m.id)}>
                  {t('cancel')}
                </button>
              )}
            </div>
            {isDownloading && pct !== null && (
              <div
                role="progressbar"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={pct}
                className={styles.progress}
              >
                {/* width is runtime-computed from downloadProgress - kept inline */}
                <div className={styles.progressBar} style={{ width: `${pct}%` }} />
              </div>
            )}
            {m.downloadState === 'error' && (
              <small className={styles.errorText}>{m.errorMessage ?? t('download_error')}</small>
            )}
          </li>
        );
      })}
    </ul>
  );
}
