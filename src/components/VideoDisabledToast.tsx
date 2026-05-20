/**
 * C3 - VideoDisabledToast
 *
 * A fixed bottom-right toast that appears once per scene change when
 * `videoEnabled === false` and `sceneTransitionsEnabled === true`.
 *
 * Auto-dismisses after 8 seconds. Two buttons:
 *   - "Enable video" - calls onOpenVideoSettings.
 *   - "Dismiss"      - hides the toast immediately.
 *
 * The toast never appears on initial mount - it only fires on genuine scene-
 * name changes after the component is mounted (same pattern as
 * SceneTransitionOverlay). A single scene never triggers multiple toasts on
 * re-renders.
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

const AUTO_DISMISS_MS = 8_000;

export type VideoDisabledToastProps = {
  videoEnabled: boolean;
  sceneTransitionsEnabled: boolean;
  /** Current scene name; any change triggers the toast when conditions are met. */
  sceneName: string | null;
  onOpenVideoSettings: () => void;
};

export function VideoDisabledToast({
  videoEnabled,
  sceneTransitionsEnabled,
  sceneName,
  onOpenVideoSettings,
}: VideoDisabledToastProps) {
  const { t } = useTranslation('common');

  // Never show on initial mount - only react to genuine scene-name changes.
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<number | null>(null);
  // Consume the initial scene name unconditionally so the first real change
  // is detected against it (mirrors SceneTransitionOverlay).
  const lastTriggeredScene = useRef<string | null>(sceneName);

  const dismiss = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setVisible(false);
  }, []);

  // Start the auto-dismiss timer when the toast becomes visible.
  useEffect(() => {
    if (!visible) return;
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current);
    }
    timerRef.current = window.setTimeout(() => {
      setVisible(false);
      timerRef.current = null;
    }, AUTO_DISMISS_MS);
    return () => {
      if (timerRef.current !== null) {
        window.clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [visible]);

  // React to scene-name changes after initial mount.
  useEffect(() => {
    if (sceneName === lastTriggeredScene.current) return;
    lastTriggeredScene.current = sceneName;
    // Show the toast only when: video is disabled AND scene transitions are
    // enabled AND we actually have a scene name to transition to.
    if (videoEnabled || !sceneTransitionsEnabled || sceneName === null) return;
    setVisible(true);
  }, [sceneName, videoEnabled, sceneTransitionsEnabled]);

  if (!visible) return null;

  return (
    <div className="dm-video-disabled-toast" role="status" aria-live="polite">
      <p className="dm-video-disabled-toast-msg">{t('video_disabled_cta')}</p>
      <div className="dm-video-disabled-toast-actions">
        <button
          type="button"
          className="dm-btn dm-btn-primary"
          onClick={() => {
            dismiss();
            onOpenVideoSettings();
          }}
        >
          {t('video_enable_settings')}
        </button>
        <button type="button" className="dm-btn dm-btn-secondary" onClick={dismiss}>
          {t('dismiss')}
        </button>
      </div>
    </div>
  );
}
