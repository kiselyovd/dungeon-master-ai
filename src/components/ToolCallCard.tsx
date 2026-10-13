import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { publicToolOutcome } from '../features/agent-turn/model/publicToolOutcome';
import { useMediaImage } from '../features/media/useMediaImage';
import type { ToolLogEntry } from '../state/toolLog';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';
import styles from './ToolCallCard.module.css';

interface Props {
  entry: ToolLogEntry;
  /** User-facing label for the tool action. Shown in place of raw tool name when provided. */
  label?: string;
  onInspect?: (() => void) | undefined;
}

const IMAGE_TOOLS = new Set(['generate_map', 'generate_illustration']);
const VIDEO_TOOLS = new Set(['generate_video']);

export function ToolCallCard({ entry, label, onInspect }: Props) {
  const {
    toolName,
    result,
    isError,
    imageDataUrl: legacyImageUrl,
    imageKind,
    imageSource,
    videoDataUrl,
  } = entry;
  const campaignId = useStore((s) => s.session.activeCampaignId);
  const media = useMediaImage(
    entry.imageMediaRef,
    campaignId ? { kind: 'campaign', campaign_id: campaignId } : null,
  );
  const imageDataUrl = entry.imageMediaRef ? media.url : legacyImageUrl;
  const pending = result === null && !isError;
  const { t } = useTranslation('agent');
  const { t: tt } = useTranslation('tools');
  const inspect = useStore((state) => state.toolLog.open);
  const publicLabel =
    label && label !== toolName ? label : tt(toolName, { defaultValue: t('tool_action') });
  const outcome = publicToolOutcome(toolName, result);
  const isImageTool = IMAGE_TOOLS.has(toolName);
  const isVideoTool = VIDEO_TOOLS.has(toolName);

  const [flashing, setFlashing] = useState(false);
  const [lightboxOpen, setLightboxOpen] = useState(false);

  useEffect(() => {
    // Image and video tools never show the text result block.
    if (isImageTool || isVideoTool) return;
    if (pending) {
      setFlashing(false);
      return;
    }
    setFlashing(!isError);
    const flashTimer = setTimeout(() => setFlashing(false), 600);
    return () => clearTimeout(flashTimer);
  }, [pending, isError, isImageTool, isVideoTool]);

  const statusKey = pending ? 'tool_pending' : isError ? 'tool_error' : 'tool_success';
  const statusLabel = pending ? 'pending' : isError ? 'error' : 'success';

  return (
    <div
      className={`${styles.card} ${!isImageTool && !isVideoTool ? styles.compact : ''} ${isError ? styles.cardError : ''} ${flashing ? styles.cardFlash : ''}`}
      data-testid={`tool-call-card-${entry.id}`}
      data-tool-name={toolName}
      data-status={statusLabel}
      data-image-kind={imageKind}
      data-image-source={imageSource?.type}
      data-image-asset-id={imageSource?.type === 'bundled' ? imageSource.assetId : undefined}
    >
      <div className={styles.header}>
        <span className={styles.toolName}>{publicLabel}</span>
        <span className={`${styles.statusBadge} ${styles[`status_${statusLabel}`]}`}>
          {t(statusKey)}
        </span>
        <button
          type="button"
          className={styles.inspectButton}
          onClick={onInspect ?? inspect}
          aria-label={t('inspect_action')}
          title={t('inspect_action')}
        >
          <Icons.ChevronRight size={14} />
        </button>
      </div>

      {isVideoTool ? (
        <div className={styles.imageBody}>
          {pending && !isError && (
            <div className={styles.drawing} data-testid="tool-drawing">
              <span className={styles.drawingShimmer} aria-hidden="true" />
              <span>{t('drawing_video')}</span>
            </div>
          )}
          {!pending && isError && <span>{t('outcome_failed')}</span>}
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
          {!pending && isError && <span>{t('outcome_failed')}</span>}
          {!pending && !isError && imageDataUrl && (
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
          )}
          {!pending && !isError && !imageDataUrl && (
            <div className={styles.drawing}>
              <span>{media.loading ? t('image_loading') : t('image_unavailable')}</span>
              {entry.imageMediaRef && media.error && (
                <button type="button" onClick={media.retry}>
                  {t('image_retry')}
                </button>
              )}
            </div>
          )}
        </div>
      ) : (
        <div className={styles.body}>
          {pending ? (
            <span
              className={styles.pendingDot}
              data-testid="tool-pending-indicator"
              aria-hidden="true"
            />
          ) : isError ? (
            <span>{t('outcome_failed')}</span>
          ) : outcome.key !== 'outcome_done' ? (
            <span>{t(outcome.key, outcome.values)}</span>
          ) : null}
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
