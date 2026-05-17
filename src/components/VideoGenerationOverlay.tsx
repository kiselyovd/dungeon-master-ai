/**
 * M7-DM: progress overlay rendered atop VttCanvas while LTX-Video generates
 * a clip in `live` or `race` mode. Hidden in idle/done states; the parent
 * decides when to start playback.
 */
import { useTranslation } from 'react-i18next';

export type VideoGenerationOverlayProps = {
  status: 'idle' | 'starting' | 'progress' | 'done' | 'error';
  percent: number | null;
  etaSeconds: number | null;
  error: string | null;
  onCancel?: () => void;
};

export function VideoGenerationOverlay({
  status,
  percent,
  etaSeconds,
  error,
  onCancel,
}: VideoGenerationOverlayProps) {
  const { t } = useTranslation('common');
  if (status === 'idle' || status === 'done') return null;
  const pct = percent != null ? Math.round(percent * 100) : null;
  return (
    <div className="dm-video-overlay" role="status" aria-live="polite">
      <div className="dm-video-overlay-card">
        {status === 'starting' && <p>{t('video_overlay_starting')}</p>}
        {status === 'progress' && (
          <>
            <p>
              {pct != null
                ? t('video_overlay_progress', { percent: pct })
                : t('video_overlay_starting')}
            </p>
            <div className="dm-video-overlay-bar">
              <div className="dm-video-overlay-bar-fill" style={{ width: `${pct ?? 0}%` }} />
            </div>
            {etaSeconds != null && etaSeconds > 0 && (
              <p className="dm-video-overlay-eta">
                {t('video_overlay_eta', { seconds: etaSeconds })}
              </p>
            )}
          </>
        )}
        {status === 'error' && (
          <p className="dm-video-overlay-error">{error ?? t('video_overlay_error_generic')}</p>
        )}
        {onCancel && (
          <button type="button" onClick={onCancel} className="dm-btn-secondary">
            {t('cancel')}
          </button>
        )}
      </div>
    </div>
  );
}
