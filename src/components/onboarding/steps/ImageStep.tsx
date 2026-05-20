/**
 * ImageStep - step 4 of 6 in the onboarding flow (image provider config).
 *
 * Fleshed out in E4. Behavior is fully driven by the selected preset:
 *   - local-only:       download SDXL-Turbo; progress bar; already-installed detection.
 *   - hybrid:           same as local-only (SDXL-Turbo download; sets imagePreset 'balanced').
 *                       NOTE: The frontend ModelId enum only has 'sdxl_turbo' for images;
 *                       there is no 'sdxl_lightning_base' or equivalent in src/state/localMode.ts.
 *                       We use sdxl_turbo as the downloadable model but set imagePreset='balanced'
 *                       so the backend routes to SDXL-Lightning at runtime. See DONE_WITH_CONCERNS.
 *   - cloud-cinematic:  Replicate API key input; persist on Continue.
 *   - manual:           auto-advances on mount (no user action required).
 *   - text-only:        never rendered (computeSteps skips this step entirely).
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { fetchLocalLlmManifest } from '../../../api/localLlm';
import { postSettingsV2 } from '../../../api/settings';
import { useModelDownload } from '../../../hooks/useModelDownload';
import { useStore } from '../../../state/useStore';
import { Icons } from '../../../ui/Icons';
import type { PresetId } from '../presets';

// Wire id and ModelId are identical for SDXL-Turbo (both 'sdxl_turbo').
const SDXL_TURBO_ID = 'sdxl_turbo' as const;

export interface ImageStepProps {
  titleId: string;
  preset: PresetId;
  onBack: () => void;
  onNext: () => void;
}

// ---------------------------------------------------------------------------
// Shared skip handler used by local + hybrid variants
// ---------------------------------------------------------------------------

async function handleSkip(onNext: () => void): Promise<void> {
  useStore.getState().settings.setImageEnabled(false);
  await postSettingsV2(useStore.getState().settings);
  onNext();
}

// ---------------------------------------------------------------------------
// Local-only variant (imagePreset='fast') and Hybrid variant (imagePreset='balanced')
// Both download sdxl_turbo - see module comment for Hybrid infra note.
// ---------------------------------------------------------------------------

interface LocalImageStepProps extends Omit<ImageStepProps, 'preset'> {
  targetPreset: 'fast' | 'balanced';
}

function LocalImageStep({ onBack, onNext, titleId, targetPreset }: LocalImageStepProps) {
  const { t } = useTranslation('onboarding');
  const { start } = useModelDownload(SDXL_TURBO_ID);
  const downloadState = useStore((s) => s.localMode.downloads[SDXL_TURBO_ID]);

  const setImageEnabled = useStore((s) => s.settings.setImageEnabled);
  const setImagePreset = useStore((s) => s.settings.setImagePreset);

  // null = loading, true = installed, false = not installed
  const [alreadyInstalled, setAlreadyInstalled] = useState<boolean | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [skipping, setSkipping] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void fetchLocalLlmManifest()
      .then((manifest) => {
        if (!cancelled) {
          setAlreadyInstalled(manifest.installed_ids.includes(SDXL_TURBO_ID));
        }
      })
      .catch(() => {
        if (!cancelled) setAlreadyInstalled(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleDownload = async () => {
    setDownloadError(null);
    try {
      await start();
    } catch (err) {
      setDownloadError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleContinue = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      setImageEnabled(true);
      setImagePreset(targetPreset);
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleSkipClick = async () => {
    setSkipping(true);
    try {
      await handleSkip(onNext);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSkipping(false);
    }
  };

  const isCompleted = downloadState.state === 'completed';
  const isDownloading = downloadState.state === 'downloading';
  const isFailed = downloadState.state === 'failed';
  const isReady = alreadyInstalled === true || isCompleted;

  const errorMessage = isFailed
    ? downloadState.state === 'failed'
      ? downloadState.reason
      : t('image_download_failed')
    : downloadError;

  let percent: number | null = null;
  if (isDownloading && downloadState.totalBytes) {
    percent = Math.round((downloadState.bytesDone / downloadState.totalBytes) * 100);
  }

  return (
    <>
      <div className="dm-onboarding-tag">{t('image_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('image_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('image_step_desc')}</p>

      {alreadyInstalled === null && (
        <p className="dm-onboarding-form-hint">{t('image_starting')}</p>
      )}

      {alreadyInstalled === true && (
        <div className="dm-onboarding-local-hint" role="status">
          {t('image_already_exists')}
        </div>
      )}

      {alreadyInstalled === false && !isCompleted && (
        <>
          {!isDownloading && !isFailed && (
            <button
              type="button"
              className="dm-onboarding-btn dm-onboarding-btn-primary dm-onboarding-btn-lg"
              onClick={() => void handleDownload()}
            >
              <Icons.Download size={16} />
              {t('image_download_cta')}
            </button>
          )}

          {isDownloading && (
            <div className="dm-onboarding-form">
              <p className="dm-onboarding-form-hint">{t('image_downloading')}</p>
              <progress
                max={100}
                value={percent ?? undefined}
                aria-label={
                  percent !== null
                    ? t('image_downloading_progress', { percent })
                    : t('image_downloading')
                }
                className="dm-onboarding-progress"
              />
              {percent !== null && (
                <p className="dm-onboarding-form-hint">
                  {t('image_downloading_progress', { percent })}
                </p>
              )}
            </div>
          )}

          {(isFailed || downloadError) && (
            <div className="dm-onboarding-form">
              <p className="dm-onboarding-form-error">{errorMessage}</p>
              <button
                type="button"
                className="dm-onboarding-btn dm-onboarding-btn-secondary"
                onClick={() => void handleDownload()}
              >
                {t('retry')}
              </button>
            </div>
          )}
        </>
      )}

      {isCompleted && (
        <div className="dm-onboarding-local-hint" role="status">
          {t('image_download_done')}
        </div>
      )}

      {alreadyInstalled !== null && (
        <p className="dm-onboarding-form-hint">{t('image_cpu_offload_enabled')}</p>
      )}

      {saveError && <p className="dm-onboarding-form-error">{saveError}</p>}

      <div className="dm-onboarding-actions">
        <button
          type="button"
          className="dm-onboarding-skip"
          onClick={() => void handleSkipClick()}
          disabled={skipping}
        >
          {t('skip')}
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
          onClick={() => void handleContinue()}
          disabled={!isReady || saving}
        >
          {t('next')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Cloud-cinematic variant (Replicate API key, imagePreset='cloud')
// ---------------------------------------------------------------------------

function CloudImageStep({ onBack, onNext, titleId }: Omit<ImageStepProps, 'preset'>) {
  const { t } = useTranslation('onboarding');
  const [apiKeyRaw, setApiKeyRaw] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [skipping, setSkipping] = useState(false);

  const setReplicateApiKey = useStore((s) => s.settings.setReplicateApiKey);
  const setImageEnabled = useStore((s) => s.settings.setImageEnabled);
  const setImagePreset = useStore((s) => s.settings.setImagePreset);

  const trimmedKey = apiKeyRaw.trim();
  const canContinue = trimmedKey.length > 0 && !saving;

  const handleContinue = async () => {
    if (!trimmedKey) return;
    setSaving(true);
    setSaveError(null);
    try {
      setReplicateApiKey(trimmedKey);
      setImageEnabled(true);
      setImagePreset('cloud');
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleSkipClick = async () => {
    setSkipping(true);
    try {
      await handleSkip(onNext);
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSkipping(false);
    }
  };

  return (
    <>
      <div className="dm-onboarding-tag">{t('image_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('image_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('image_step_desc')}</p>

      <div className="dm-onboarding-form">
        <label htmlFor="replicate-api-key" className="dm-onboarding-form-hint">
          {t('replicate_key_label')}
        </label>
        <input
          id="replicate-api-key"
          type="password"
          className="dm-onboarding-form-input"
          placeholder="r8_..."
          value={apiKeyRaw}
          onChange={(e) => setApiKeyRaw(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <p className="dm-onboarding-form-hint">{t('replicate_key_hint')}</p>
        {saveError && <p className="dm-onboarding-form-error">{saveError}</p>}
      </div>

      <div className="dm-onboarding-actions">
        <button
          type="button"
          className="dm-onboarding-skip"
          onClick={() => void handleSkipClick()}
          disabled={skipping}
        >
          {t('skip')}
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
          onClick={() => void handleContinue()}
          disabled={!canContinue}
        >
          {t('next')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}

// ---------------------------------------------------------------------------
// Manual variant - auto-advance, no UI needed
// ---------------------------------------------------------------------------

function ManualImageStep({ onNext }: Pick<ImageStepProps, 'onNext'>) {
  const didAdvance = useRef(false);

  useEffect(() => {
    if (didAdvance.current) return;
    didAdvance.current = true;
    onNext();
  }, [onNext]);

  return null;
}

// ---------------------------------------------------------------------------
// Public component - delegates to the correct variant
// ---------------------------------------------------------------------------

export function ImageStep({ titleId, preset, onBack, onNext }: ImageStepProps) {
  switch (preset) {
    case 'local-only':
      return (
        <LocalImageStep titleId={titleId} onBack={onBack} onNext={onNext} targetPreset="fast" />
      );
    case 'hybrid':
      return (
        <LocalImageStep titleId={titleId} onBack={onBack} onNext={onNext} targetPreset="balanced" />
      );
    case 'cloud-cinematic':
      return <CloudImageStep titleId={titleId} onBack={onBack} onNext={onNext} />;
    case 'manual':
      return <ManualImageStep onNext={onNext} />;
    case 'text-only':
      // computeSteps skips the image step for text-only; this branch is unreachable at runtime.
      return null;
  }
}
