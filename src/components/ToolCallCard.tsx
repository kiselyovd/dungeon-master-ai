import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ToolLogEntry } from '../state/toolLog';
import styles from './ToolCallCard.module.css';

interface Props {
  entry: ToolLogEntry;
  /** User-facing label for the tool action. Shown in place of raw tool name when provided. */
  label?: string;
}

const IMAGE_TOOLS = new Set(['generate_map', 'generate_illustration']);

export function ToolCallCard({ entry, label }: Props) {
  const { toolName, args, result, isError, round, imageDataUrl, imageKind } = entry;
  const pending = result === null;
  const { t } = useTranslation('agent');
  const isImageTool = IMAGE_TOOLS.has(toolName);

  const [displayResult, setDisplayResult] = useState<string>('...');
  const [settled, setSettled] = useState(false);
  const [flashing, setFlashing] = useState(false);
  const [lightboxOpen, setLightboxOpen] = useState(false);

  useEffect(() => {
    // Image tools never show the cycling-digits result animation.
    if (isImageTool) return;
    if (pending) {
      setSettled(false);
      setFlashing(false);
      setDisplayResult('...');
      const interval = setInterval(() => {
        const n = Math.floor(Math.random() * 20) + 1;
        setDisplayResult(String(n));
      }, 100);
      return () => clearInterval(interval);
    }
    setDisplayResult(JSON.stringify(result, null, 2));
    setSettled(true);
    setFlashing(true);
    const flashTimer = setTimeout(() => setFlashing(false), 600);
    return () => clearTimeout(flashTimer);
  }, [pending, result, isImageTool]);

  const statusKey = pending ? 'tool_pending' : isError ? 'tool_error' : 'tool_success';
  const statusLabel = pending ? 'pending' : isError ? 'error' : 'success';

  return (
    <div
      className={`${styles.card} ${isError ? styles.cardError : ''} ${flashing ? styles.cardFlash : ''}`}
      data-testid={`tool-call-card-${entry.id}`}
      data-status={statusLabel}
    >
      <div className={styles.header}>
        <span className={styles.toolName}>{label ?? toolName}</span>
        <span className={`${styles.statusBadge} ${styles[`status_${statusLabel}`]}`}>
          {t(statusKey)}
        </span>
        <span className={styles.round}>{t('round_label', { round })}</span>
      </div>

      {isImageTool ? (
        <div className={styles.imageBody}>
          {pending && !isError && (
            <div className={styles.drawing} data-testid="tool-drawing">
              <span className={styles.drawingShimmer} aria-hidden="true" />
              <span>
                {toolName === 'generate_map' ? t('drawing_map') : t('drawing_illustration')}
              </span>
            </div>
          )}
          {!pending && isError && (
            <pre className={styles.code}>{JSON.stringify(result, null, 2)}</pre>
          )}
          {!pending && !isError && imageDataUrl && (
            <>
              {imageKind === 'map' && <div className={styles.mapNote}>{t('map_updated')}</div>}
              <button
                type="button"
                className={styles.thumbButton}
                onClick={() => setLightboxOpen(true)}
                aria-label={t('image_open')}
              >
                <img
                  src={imageDataUrl}
                  alt={t('image_alt')}
                  className={styles.thumb}
                  loading="lazy"
                  decoding="async"
                />
              </button>
            </>
          )}
          {!pending && !isError && !imageDataUrl && (
            <div className={styles.drawing}>
              <span>{toolName === 'generate_map' ? t('map_updated') : t('image_alt')}</span>
            </div>
          )}
        </div>
      ) : (
        <div className={styles.body}>
          <div className={styles.section}>
            <span className={styles.label}>args</span>
            <pre className={styles.code}>{JSON.stringify(args, null, 2)}</pre>
          </div>
          <div className={styles.section}>
            <span className={styles.label}>result</span>
            <pre
              className={`${styles.code} ${pending ? styles.cycling : ''} ${settled ? styles.settled : ''}`}
            >
              {displayResult}
            </pre>
          </div>
        </div>
      )}

      {lightboxOpen && imageDataUrl && (
        <div
          className={styles.lightbox}
          role="dialog"
          aria-modal="true"
          aria-label={t('image_alt')}
          onClick={() => setLightboxOpen(false)}
          onKeyDown={(e) => e.key === 'Escape' && setLightboxOpen(false)}
        >
          <img src={imageDataUrl} alt={t('image_alt')} className={styles.lightboxImg} />
        </div>
      )}
    </div>
  );
}
