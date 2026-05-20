import { useTranslation } from 'react-i18next';
import type { MergedEntry } from '../../../state/local_llm/manifest';

export interface ManageDownloadsProps {
  models: MergedEntry[];
  onDownload: (id: string) => void;
  onDelete: (id: string) => void;
}

export function ManageDownloads({ models, onDownload, onDelete }: ManageDownloadsProps) {
  const { t } = useTranslation('local_llm');

  return (
    <ul style={{ listStyle: 'none', padding: 0, margin: 0 }}>
      {models.map((m) => {
        const pct = m.downloadProgress !== undefined ? Math.round(m.downloadProgress * 100) : null;
        const isDownloading =
          m.downloadState === 'queued' ||
          m.downloadState === 'downloading' ||
          m.downloadState === 'verifying';
        return (
          <li
            key={m.id}
            data-testid="download-row"
            style={{ padding: '8px 0', borderBottom: '1px solid rgba(212,175,55,0.1)' }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ flex: 1 }}>
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
                style={{
                  marginTop: 4,
                  height: 4,
                  background: 'rgba(212,175,55,0.1)',
                  borderRadius: 2,
                }}
              >
                <div
                  style={{
                    width: `${pct}%`,
                    height: '100%',
                    background: 'rgba(212,175,55,0.6)',
                  }}
                />
              </div>
            )}
            {m.downloadState === 'error' && (
              <small style={{ color: 'rgba(220, 100, 100, 0.9)' }}>
                {m.errorMessage ?? t('download_error')}
              </small>
            )}
          </li>
        );
      })}
    </ul>
  );
}
