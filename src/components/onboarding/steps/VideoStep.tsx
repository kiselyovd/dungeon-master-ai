/**
 * VideoStep - step 5 of 6 in the onboarding flow (video provider config).
 *
 * Fleshed out in E5. Only shown for the 'cloud-cinematic' preset.
 * Reads the existing replicateApiKey from the image step (E4). If the key
 * is present, the Enable button is actionable. If absent, Enable is disabled
 * and a hint is shown explaining that the Replicate key is required.
 *
 * Skip always works: sets videoEnabled=false, persists, advances.
 * Enable: sets videoEnabled=true + videoMode='live', persists, advances.
 * Both catch postSettingsV2 rejections and surface them inline.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { postSettingsV2 } from '../../../api/settings';
import { useStore } from '../../../state/useStore';
import { Icons } from '../../../ui/Icons';

export interface VideoStepProps {
  titleId: string;
  onBack: () => void;
  onNext: () => void;
}

export function VideoStep({ titleId, onBack, onNext }: VideoStepProps) {
  const { t } = useTranslation('onboarding');

  const replicateApiKey = useStore((s) => s.settings.replicateApiKey);
  const setVideoEnabled = useStore((s) => s.settings.setVideoEnabled);
  const setVideoMode = useStore((s) => s.settings.setVideoMode);

  const [enabling, setEnabling] = useState(false);
  const [skipping, setSkipping] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const hasKey = replicateApiKey !== null && replicateApiKey.length > 0;

  const handleEnable = async () => {
    setEnabling(true);
    setSaveError(null);
    try {
      setVideoEnabled(true);
      setVideoMode('live');
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setEnabling(false);
    }
  };

  const handleSkip = async () => {
    setSkipping(true);
    setSaveError(null);
    try {
      setVideoEnabled(false);
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSkipping(false);
    }
  };

  return (
    <>
      <div className="dm-onboarding-tag">{t('video_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('video_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('video_step_desc')}</p>

      <p className="dm-onboarding-form-hint">{t('video_cost_hint')}</p>

      {hasKey ? (
        <p className="dm-onboarding-form-hint">{t('video_key_reused')}</p>
      ) : (
        <p className="dm-onboarding-form-hint">{t('video_key_missing_hint')}</p>
      )}

      {saveError && <p className="dm-onboarding-form-error">{saveError}</p>}

      <div className="dm-onboarding-actions">
        <button
          type="button"
          className="dm-onboarding-skip"
          onClick={() => void handleSkip()}
          disabled={skipping || enabling}
        >
          {t('video_skip')}
        </button>
        <button
          type="button"
          className="dm-onboarding-btn dm-onboarding-btn-secondary"
          onClick={onBack}
          aria-label={t('back')}
        >
          <Icons.ChevronLeft size={14} />
          {t('back')}
        </button>
        <button
          type="button"
          className="dm-onboarding-btn dm-onboarding-btn-primary"
          onClick={() => void handleEnable()}
          disabled={!hasKey || enabling || skipping}
        >
          {t('video_enable_cta')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}
