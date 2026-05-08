import { useTranslation } from 'react-i18next';
import type { DownloadState, ModelId } from '../state/localMode';
import styles from './ModelDownloadCard.module.css';

interface Props {
  modelId: ModelId;
  displayName: string;
  sizeBytes: number;
  vramBytes?: number | undefined;
  vramWarning?: string | undefined;
  state: DownloadState;
  active: boolean;
  onSelect: () => void;
  onDownload: () => void;
  onDelete: () => void;
}

const formatGb = (bytes: number) => `${(bytes / 1024 ** 3).toFixed(1)} GB`;

export function ModelDownloadCard(p: Props) {
  const { t } = useTranslation('local_mode');
  const progressPct =
    p.state.state === 'downloading' && p.state.totalBytes
      ? Math.min(100, Math.round((p.state.bytesDone / p.state.totalBytes) * 100))
      : 0;

  return (
    <div className={`${styles.card} ${p.active ? styles.active : ''}`} role="group">
      <div className={styles.head}>
        <strong>{p.displayName}</strong>
        <span className={styles.size}>{formatGb(p.sizeBytes)}</span>
        {p.vramBytes !== undefined && (
          <span className={styles.size}>VRAM ~{formatGb(p.vramBytes)}</span>
        )}
        {p.vramWarning && <span className={styles.warn}>{p.vramWarning}</span>}
      </div>

      {p.state.state === 'downloading' && (
        <progress
          className={styles.progress}
          role="progressbar"
          value={progressPct}
          max={100}
          aria-valuenow={progressPct}
          aria-valuemin={0}
          aria-valuemax={100}
        />
      )}

      <div className={styles.actions}>
        {p.state.state === 'idle' && (
          <button type="button" onClick={p.onDownload}>
            {t('download')}
          </button>
        )}
        {p.state.state === 'downloading' && (
          <button type="button" onClick={p.onDelete}>
            {t('cancel')}
          </button>
        )}
        {p.state.state === 'completed' && (
          <>
            <button type="button" onClick={p.onSelect} aria-pressed={p.active}>
              {p.active ? t('in_use') : t('use')}
            </button>
            <button type="button" onClick={p.onDelete}>
              {t('delete')}
            </button>
          </>
        )}
        {p.state.state === 'failed' && (
          <>
            <span role="alert" className={styles.error}>
              {p.state.reason}
            </span>
            <button type="button" onClick={p.onDownload}>
              {t('download')}
            </button>
          </>
        )}
      </div>
    </div>
  );
}
