/**
 * ChatStep - step 3 of 6 in the onboarding flow (chat provider config).
 *
 * Fleshed out in E3. Behavior is fully driven by the selected preset:
 *   - local-only:        download Qwen3.5-4B; progress bar; already-installed detection.
 *   - cloud-cinematic:  OpenAI-compatible cloud (base URL + key + model); persist on Continue.
 *   - hybrid:           same as cloud-cinematic.
 *   - text-only:        same cloud form + Skip option (no persist).
 *   - manual:           auto-advances on mount (no user action required).
 *
 * Cloud chat uses the generic OpenAI-compatible provider with OpenRouter as the
 * recommended default base URL (native Anthropic was removed in M11 Batch D.5).
 */

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { fetchLocalLlmManifest } from '../../../api/localLlm';
import { fetchLocalRuntimeStatus, startLocalRuntimes } from '../../../api/localRuntime';
import { postSettingsV2 } from '../../../api/settings';
import { useModelDownload } from '../../../hooks/useModelDownload';
import {
  DEFAULT_LOCAL_CONTEXT_WINDOW,
  OPENROUTER_BASE_URL,
  parseApiKey,
  parseBaseUrl,
} from '../../../state/providers';
import { useStore } from '../../../state/useStore';
import { Icons } from '../../../ui/Icons';
import { HfTokenModal } from '../../settings/local-llm/HfTokenModal';
import type { PresetId } from '../presets';

// Wire id for the 4B model in the local-llm manifest.
const QWEN_4B_WIRE_ID = 'qwen3.5-4b';
const QWEN_4B_MODEL_ID = 'qwen3_5_4b' as const;

export interface ChatStepProps {
  titleId: string;
  preset: PresetId;
  onBack: () => void;
  onNext: () => void;
}

// ---------------------------------------------------------------------------
// Local-only variant
// ---------------------------------------------------------------------------

function LocalOnlyChatStep({ onBack, onNext, titleId }: Omit<ChatStepProps, 'preset'>) {
  const { t } = useTranslation('onboarding');
  const { start } = useModelDownload(QWEN_4B_MODEL_ID);
  const downloadState = useStore((s) => s.localMode.downloads[QWEN_4B_MODEL_ID]);

  const setProviderConfig = useStore((s) => s.settings.setProviderConfig);
  const setActiveProvider = useStore((s) => s.settings.setActiveProvider);

  // null = loading, true = installed, false = not installed
  const [alreadyInstalled, setAlreadyInstalled] = useState<boolean | null>(null);
  // Single error channel - covers both SSE failures and start() rejections.
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [tokenModalOpen, setTokenModalOpen] = useState(false);
  const authRequired = downloadState.state === 'failed' && downloadState.authRequired;

  useEffect(() => {
    let cancelled = false;
    void fetchLocalLlmManifest()
      .then((manifest) => {
        if (!cancelled) {
          setAlreadyInstalled(manifest.installed_ids.includes(QWEN_4B_WIRE_ID));
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
      // setProviderConfig and setActiveProvider must run synchronously BEFORE
      // useStore.getState().settings is read below - do not reorder these calls.
      setProviderConfig({
        kind: 'local-mistralrs',
        modelPath: QWEN_4B_MODEL_ID,
        contextWindow: DEFAULT_LOCAL_CONTEXT_WINDOW,
      });
      setActiveProvider('local-mistralrs');
      useStore.getState().localMode.setEnabled(true);
      useStore.getState().localMode.selectModel(QWEN_4B_MODEL_ID);
      // Onboarding only DOWNLOADS the model; the runtime is still 'off', so
      // postSettingsV2 -> local-mistralrs slice would throw "runtime not ready"
      // (no port). Start the sidecar and refresh the store snapshot (the poll
      // hook isn't mounted here) so the synchronous settings read sees 'ready'
      // with a port before we POST. [E1]
      await startLocalRuntimes();
      const snap = await fetchLocalRuntimeStatus();
      useStore.getState().localMode.setRuntimeStatus(snap);
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const isCompleted = downloadState.state === 'completed';
  const isDownloading = downloadState.state === 'downloading';
  const isFailed = downloadState.state === 'failed';
  const isReady = alreadyInstalled === true || isCompleted;

  // Derive the single error to display - SSE failure takes precedence when set.
  const errorMessage = isFailed
    ? downloadState.state === 'failed'
      ? downloadState.reason
      : t('chat_local_download_failed')
    : downloadError;

  let percent: number | null = null;
  if (isDownloading && downloadState.totalBytes) {
    percent = Math.round((downloadState.bytesDone / downloadState.totalBytes) * 100);
  }

  return (
    <>
      <div className="dm-onboarding-tag">{t('chat_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('chat_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('chat_local_desc')}</p>

      {alreadyInstalled === null && (
        <p className="dm-onboarding-form-hint">{t('chat_local_starting')}</p>
      )}

      {alreadyInstalled === true && (
        <div className="dm-onboarding-local-hint" role="status">
          {t('chat_local_already_exists')}
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
              {t('chat_local_download_cta')}
            </button>
          )}

          {isDownloading && (
            <div className="dm-onboarding-form">
              <p className="dm-onboarding-form-hint">{t('chat_local_downloading')}</p>
              <progress
                max={100}
                value={percent ?? undefined}
                aria-label={
                  percent !== null
                    ? t('chat_local_downloading_progress', { percent })
                    : t('chat_local_downloading')
                }
                className="dm-onboarding-progress"
              />
              {percent !== null && (
                <p className="dm-onboarding-form-hint">
                  {t('chat_local_downloading_progress', { percent })}
                </p>
              )}
            </div>
          )}

          {(isFailed || downloadError) && (
            <div className="dm-onboarding-form">
              <p className="dm-onboarding-form-error">{errorMessage}</p>
              {authRequired && (
                <button
                  type="button"
                  className="dm-onboarding-btn dm-onboarding-btn-secondary"
                  onClick={() => setTokenModalOpen(true)}
                >
                  {t('chat_local_add_hf_token')}
                </button>
              )}
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
          {t('chat_local_download_done')}
        </div>
      )}

      {saveError && <p className="dm-onboarding-form-error">{saveError}</p>}

      <div className="dm-onboarding-actions">
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
      <HfTokenModal
        open={tokenModalOpen}
        onClose={() => setTokenModalOpen(false)}
        onSaved={() => {
          setTokenModalOpen(false);
          void handleDownload();
        }}
      />
    </>
  );
}

// ---------------------------------------------------------------------------
// Cloud / Hybrid variant (OpenAI-compatible: base URL + key + model)
// ---------------------------------------------------------------------------

function CloudChatStep({
  onBack,
  onNext,
  titleId,
  allowSkip,
}: Omit<ChatStepProps, 'preset'> & { allowSkip: boolean }) {
  const { t } = useTranslation('onboarding');
  const [baseUrlRaw, setBaseUrlRaw] = useState(OPENROUTER_BASE_URL);
  const [apiKeyRaw, setApiKeyRaw] = useState('');
  const [modelRaw, setModelRaw] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  const setProviderConfig = useStore((s) => s.settings.setProviderConfig);
  const setActiveProvider = useStore((s) => s.settings.setActiveProvider);

  const parsedBaseUrl = parseBaseUrl(baseUrlRaw);
  const parsedKey = parseApiKey(apiKeyRaw);
  const trimmedModel = modelRaw.trim();
  const canContinue =
    parsedBaseUrl !== null && parsedKey !== null && trimmedModel !== '' && !saving;

  const handleContinue = async () => {
    if (!parsedBaseUrl || !parsedKey || trimmedModel === '') return;
    setSaving(true);
    setSaveError(null);
    try {
      // setProviderConfig and setActiveProvider must run synchronously BEFORE
      // useStore.getState().settings is read below - do not reorder these calls.
      setProviderConfig({
        kind: 'openai-compat',
        baseUrl: parsedBaseUrl,
        apiKey: parsedKey,
        model: trimmedModel,
      });
      setActiveProvider('openai-compat');
      await postSettingsV2(useStore.getState().settings);
      onNext();
    } catch (err) {
      setSaveError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const descKey = allowSkip ? 'chat_text_only_desc' : 'chat_cloud_desc';

  return (
    <>
      <div className="dm-onboarding-tag">{t('chat_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('chat_step_title')}
      </h1>
      <p className="dm-onboarding-desc">{t(descKey)}</p>

      <div className="dm-onboarding-form">
        <label htmlFor="chat-base-url" className="dm-onboarding-form-hint">
          {t('cloud_base_url_label')}
        </label>
        <input
          id="chat-base-url"
          type="text"
          className="dm-onboarding-form-input"
          placeholder={t('cloud_base_url_placeholder')}
          value={baseUrlRaw}
          onChange={(e) => setBaseUrlRaw(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <p className="dm-onboarding-form-hint">{t('cloud_base_url_hint')}</p>

        <label htmlFor="chat-api-key" className="dm-onboarding-form-hint">
          {t('cloud_api_key_label')}
        </label>
        <input
          id="chat-api-key"
          type="password"
          className="dm-onboarding-form-input"
          placeholder={t('cloud_api_key_placeholder')}
          value={apiKeyRaw}
          onChange={(e) => setApiKeyRaw(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <p className="dm-onboarding-form-hint">{t('cloud_api_key_hint')}</p>

        <label htmlFor="chat-model" className="dm-onboarding-form-hint">
          {t('cloud_model_label')}
        </label>
        <input
          id="chat-model"
          type="text"
          className="dm-onboarding-form-input"
          placeholder={t('cloud_model_placeholder')}
          value={modelRaw}
          onChange={(e) => setModelRaw(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        {saveError && <p className="dm-onboarding-form-error">{saveError}</p>}
      </div>

      <div className="dm-onboarding-actions">
        {allowSkip && (
          <button type="button" className="dm-onboarding-skip" onClick={onNext}>
            {t('skip')}
          </button>
        )}
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

function ManualChatStep({ onNext }: Pick<ChatStepProps, 'onNext'>) {
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

export function ChatStep({ titleId, preset, onBack, onNext }: ChatStepProps) {
  switch (preset) {
    case 'local-only':
      return <LocalOnlyChatStep titleId={titleId} onBack={onBack} onNext={onNext} />;
    case 'cloud-cinematic':
    case 'hybrid':
      return <CloudChatStep titleId={titleId} onBack={onBack} onNext={onNext} allowSkip={false} />;
    case 'text-only':
      return <CloudChatStep titleId={titleId} onBack={onBack} onNext={onNext} allowSkip={true} />;
    case 'manual':
      return <ManualChatStep onNext={onNext} />;
  }
}
