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
const VIDEO_TOOLS = new Set(['generate_video']);

export function ToolCallCard({ entry, label }: Props) {
  const { toolName, args, result, isError, round, imageDataUrl, imageKind, videoDataUrl } = entry;
  const pending = result === null;
  const { t } = useTranslation('agent');
  const isImageTool = IMAGE_TOOLS.has(toolName);
  const isVideoTool = VIDEO_TOOLS.has(toolName);

  const [displayResult, setDisplayResult] = useState<string | null>(null);
  const [settled, setSettled] = useState(false);
  const [flashing, setFlashing] = useState(false);
  const [lightboxOpen, setLightboxOpen] = useState(false);

  useEffect(() => {
    // Image and video tools never show the text result block.
    if (isImageTool || isVideoTool) return;
    if (pending) {
      setSettled(false);
      setFlashing(false);
      setDisplayResult(null);
      return;
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

      {isVideoTool ? (
        <div className={styles.imageBody}>
          {pending && !isError && (
            <div className={styles.drawing} data-testid="tool-drawing">
              <span className={styles.drawingShimmer} aria-hidden="true" />
              <span>{t('drawing_video')}</span>
            </div>
          )}
          {!pending && isError && (
            <pre className={styles.code}>{JSON.stringify(result, null, 2)}</pre>
          )}
          {!pending && !isError && videoDataUrl && (
            <video controls src={videoDataUrl} aria-label={t('video_alt')} className={styles.thumb}>
              {/* Generated clips carry no spoken dialogue; an empty captions track
                  satisfies the a11y requirement without misrepresenting content. */}
              <track kind="captions" />
            </video>
          )}
          {!pending && !isError && !videoDataUrl && (
            <div className={styles.drawing}>
              <span>{t('video_alt')}</span>
            </div>
          )}
        </div>
      ) : isImageTool ? (
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
            {pending ? (
              <span
                className={styles.pendingDot}
                data-testid="tool-pending-indicator"
                aria-hidden="true"
              />
            ) : (
              <pre className={`${styles.code} ${settled ? styles.settled : ''}`}>
                {displayResult}
              </pre>
            )}
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
