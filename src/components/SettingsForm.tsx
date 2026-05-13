import { type KeyboardEvent, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocalRuntimeStatus } from '../hooks/useLocalRuntimeStatus';
import { useModelDownload } from '../hooks/useModelDownload';
import type { ModelId, VramStrategy } from '../state/localMode';
import {
  type AnthropicConfig,
  assertNeverProvider,
  DEFAULT_ANTHROPIC_MODEL,
  DEFAULT_LOCAL_CONTEXT_WINDOW,
  type LocalMistralRsConfig,
  type OpenaiCompatConfig,
  type ProviderConfig,
  type ProviderKind,
  parseApiKey,
  parseBaseUrl,
} from '../state/providers';
import { useStore } from '../state/useStore';
import { Field } from '../ui/Field';
import { ModelDownloadCard } from './ModelDownloadCard';
import { RuntimeStatusPill } from './RuntimeStatusPill';
import styles from './SettingsForm.module.css';

const PROVIDER_KINDS: readonly ProviderKind[] = ['anthropic', 'openai-compat', 'local-mistralrs'];

type Tab = 'provider' | 'model';

export interface SettingsSubmission {
  provider: ProviderConfig;
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string;
}

interface SettingsFormProps {
  /** Called when the user saves; consumer typically POSTs to /settings + closes the modal. */
  onSubmit: (submission: SettingsSubmission) => Promise<void> | void;
  /** Form id so an external Save button can `requestSubmit()` from outside the form tree. */
  formId?: string;
  /** Optional callback invoked when the user wants to re-create their character. */
  onRequestCharacterRecreate?: () => void;
}

/**
 * Multi-provider Settings form.
 *
 * The form keeps each provider kind's draft state independent so the user
 * can flip between Anthropic, OpenAI-compat, and local-mistralrs without
 * losing what they've typed. The local-mistralrs sub-form mirrors the
 * Ctrl+Shift+M LocalModeModal (model picker + VRAM strategy + runtime
 * controls) so the embedded provider can be configured from Settings.
 *
 * M3 added a tab bar at the top: the Provider tab keeps the provider
 * picker + per-kind sub-form + language selects, and the Model tab exposes
 * the agent-loop knobs (system prompt, temperature, Replicate API key)
 * wired to `POST /agent-settings`. M4 lights up local-mistralrs as the
 * third provider option (radio-card UI deferred to a later polish pass).
 */
export function SettingsForm({ onSubmit, formId, onRequestCharacterRecreate }: SettingsFormProps) {
  const { t } = useTranslation('settings');
  const slice = useStore((s) => s.settings);
  // Read the live ModelId from the localMode slice at submit time. We read
  // from getState() rather than subscribing because the selection only
  // matters at the moment Save fires, and a subscription here would force
  // every form rerender to also rebuild on unrelated download progress.
  const localModeSelection = useStore((s) => s.localMode.selectedLlm);

  const [activeTab, setActiveTab] = useState<Tab>('provider');
  // Drafts and activeKind are seeded once from the slice on mount and live
  // independently afterwards. SettingsModal unmounts the form on close
  // (`if (!open) return null`), so the next open re-seeds from the latest slice.
  // Without this, a useEffect([slice]) would reset drafts on every store write,
  // wiping in-progress edits whenever any other code path touches the settings slice.
  const [activeKind, setActiveKind] = useState<ProviderKind>(slice.activeProvider);
  const [drafts, setDrafts] = useState<DraftState>(() => initialDrafts(slice));
  const [submitting, setSubmitting] = useState(false);
  const [errors, setErrors] = useState<DraftErrors>({});

  const onTabKeyDown = (e: KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
      e.preventDefault();
      setActiveTab((prev) => (prev === 'provider' ? 'model' : 'provider'));
      // Move focus to the newly-active tab. Note: `activeTab` here still
      // holds the previous value at the moment the keypress fires, so we
      // focus the OPPOSITE tab. The rAF runs after React updates the DOM,
      // by which time `tabIndex` on the now-active tab is `0`.
      const targetId = activeTab === 'provider' ? 'settings-tab-model' : 'settings-tab-provider';
      requestAnimationFrame(() => {
        document.getElementById(targetId)?.focus();
      });
    }
  };

  const onSave = async () => {
    const result = buildConfig(activeKind, drafts, localModeSelection);
    if (!result.ok) {
      // If the validation error lives in the Provider tab, surface it by
      // switching back to that tab so the inline message is visible.
      setActiveTab('provider');
      setErrors(result.errors);
      return;
    }
    setErrors({});
    setSubmitting(true);
    try {
      await onSubmit({
        provider: result.config,
        uiLanguage: drafts.uiLanguage,
        narrationLanguage: drafts.narrationLanguage,
        systemPrompt: drafts.systemPrompt,
        temperature: drafts.temperature,
        replicateApiKey: drafts.replicateApiKey,
      });
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form
      id={formId}
      className={styles.form}
      onSubmit={(e) => {
        e.preventDefault();
        void onSave();
      }}
    >
      <div className={styles.tabNav} role="tablist">
        <button
          type="button"
          role="tab"
          id="settings-tab-provider"
          aria-controls="settings-panel-provider"
          aria-selected={activeTab === 'provider'}
          tabIndex={activeTab === 'provider' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'provider' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('provider')}
        >
          {t('tab_provider')}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-tab-model"
          aria-controls="settings-panel-model"
          aria-selected={activeTab === 'model'}
          tabIndex={activeTab === 'model' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'model' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('model')}
        >
          {t('tab_model')}
        </button>
      </div>

      {activeTab === 'provider' && (
        <div role="tabpanel" id="settings-panel-provider" aria-labelledby="settings-tab-provider">
          <Field label={t('provider_label')}>
            {({ id }) => (
              <select
                id={id}
                value={activeKind}
                onChange={(e) => setActiveKind(e.target.value as ProviderKind)}
                className={styles.fullWidth}
              >
                {PROVIDER_KINDS.map((k) => (
                  <option key={k} value={k}>
                    {t(
                      `provider_${k.replace('-', '_')}` as
                        | 'provider_anthropic'
                        | 'provider_openai_compat'
                        | 'provider_local_mistralrs',
                    )}
                  </option>
                ))}
              </select>
            )}
          </Field>

          {activeKind === 'anthropic' && (
            <AnthropicFields
              draft={drafts.anthropic}
              errors={errors}
              onChange={(d) => setDrafts((prev) => ({ ...prev, anthropic: d }))}
            />
          )}

          {activeKind === 'openai-compat' && (
            <OpenaiCompatFields
              draft={drafts['openai-compat']}
              errors={errors}
              onChange={(d) => setDrafts((prev) => ({ ...prev, 'openai-compat': d }))}
            />
          )}

          {activeKind === 'local-mistralrs' && <LocalMistralRsFields />}

          <div className={styles.languages}>
            <Field label={t('language_ui_label')}>
              {({ id }) => (
                <LanguageSelect
                  id={id}
                  value={drafts.uiLanguage}
                  onChange={(uiLanguage) => setDrafts((prev) => ({ ...prev, uiLanguage }))}
                />
              )}
            </Field>
            <Field label={t('language_narration_label')}>
              {({ id }) => (
                <LanguageSelect
                  id={id}
                  value={drafts.narrationLanguage}
                  onChange={(narrationLanguage) =>
                    setDrafts((prev) => ({ ...prev, narrationLanguage }))
                  }
                />
              )}
            </Field>
          </div>

          {onRequestCharacterRecreate && (
            <section
              style={{
                marginTop: 24,
                paddingTop: 16,
                borderTop: '1px solid rgba(212,175,55,0.15)',
              }}
            >
              <h3>{t('character_section_title')}</h3>
              <button
                type="button"
                className="dm-onboarding-btn dm-onboarding-btn-secondary"
                onClick={() => onRequestCharacterRecreate()}
              >
                {t('recreate_character')}
              </button>
            </section>
          )}
        </div>
      )}

      {activeTab === 'model' && (
        <div role="tabpanel" id="settings-panel-model" aria-labelledby="settings-tab-model">
          <ModelTab
            draft={{
              systemPrompt: drafts.systemPrompt,
              temperature: drafts.temperature,
              replicateApiKey: drafts.replicateApiKey,
            }}
            onChange={(patch) => setDrafts((prev) => ({ ...prev, ...patch }))}
          />
        </div>
      )}

      <input type="submit" hidden disabled={submitting} />
    </form>
  );
}

// ---- Sub-forms per provider --------------------------------------------

interface AnthropicDraft {
  apiKey: string;
  model: string;
}

function AnthropicFields({
  draft,
  errors,
  onChange,
}: {
  draft: AnthropicDraft;
  errors: DraftErrors;
  onChange: (d: AnthropicDraft) => void;
}) {
  const { t } = useTranslation('settings');
  return (
    <>
      <Field label={t('api_key_label')} error={errors.apiKey}>
        {(p) => (
          <input
            {...p}
            type="password"
            autoComplete="off"
            value={draft.apiKey}
            onChange={(e) => onChange({ ...draft, apiKey: e.target.value })}
            placeholder={t('api_key_placeholder')}
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={t('model_label')} error={errors.model}>
        {(p) => (
          <input
            {...p}
            type="text"
            value={draft.model}
            onChange={(e) => onChange({ ...draft, model: e.target.value })}
            placeholder={DEFAULT_ANTHROPIC_MODEL}
            className={styles.fullWidth}
          />
        )}
      </Field>
    </>
  );
}

interface OpenaiCompatDraft {
  baseUrl: string;
  apiKey: string;
  model: string;
}

function OpenaiCompatFields({
  draft,
  errors,
  onChange,
}: {
  draft: OpenaiCompatDraft;
  errors: DraftErrors;
  onChange: (d: OpenaiCompatDraft) => void;
}) {
  const { t } = useTranslation('settings');
  return (
    <>
      <Field label={t('base_url_label')} helper={t('base_url_helper')} error={errors.baseUrl}>
        {(p) => (
          <input
            {...p}
            type="url"
            value={draft.baseUrl}
            onChange={(e) => onChange({ ...draft, baseUrl: e.target.value })}
            placeholder="http://localhost:1234/v1"
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={t('api_key_label')} error={errors.apiKey}>
        {(p) => (
          <input
            {...p}
            type="password"
            autoComplete="off"
            value={draft.apiKey}
            onChange={(e) => onChange({ ...draft, apiKey: e.target.value })}
            placeholder={t('api_key_placeholder_openai')}
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={t('model_label')} error={errors.model}>
        {(p) => (
          <input
            {...p}
            type="text"
            value={draft.model}
            onChange={(e) => onChange({ ...draft, model: e.target.value })}
            placeholder="qwen3-1.7b"
            className={styles.fullWidth}
          />
        )}
      </Field>
    </>
  );
}

interface LocalLlmEntry {
  id: ModelId;
  name: string;
  size: number;
  vram: number;
  warn?: string;
}

const LOCAL_LLMS: readonly LocalLlmEntry[] = [
  { id: 'qwen3_5_0_8b', name: 'Qwen3.5-0.8B Q4_K_M', size: 600e6, vram: 900e6 },
  { id: 'qwen3_5_2b', name: 'Qwen3.5-2B Q4_K_M', size: 1.5e9, vram: 2.0e9 },
  { id: 'qwen3_5_4b', name: 'Qwen3.5-4B Q4_K_M', size: 3.0e9, vram: 2.5e9 },
  {
    id: 'qwen3_5_9b',
    name: 'Qwen3.5-9B Q4_K_M',
    size: 6.5e9,
    vram: 5.5e9,
    warn: 'requires VRAM swap with image-gen',
  },
];

const RUNTIME_RESET_DELAY_MS = 3500;
type RuntimeActionStatus = 'idle' | 'pending' | 'error';

/**
 * `local-mistralrs` provider editor inside the Settings -> Provider tab.
 *
 * Mirrors the LocalModeModal (Ctrl+Shift+M) so the embedded provider can be
 * configured without leaving Settings: pick a Qwen variant, choose a VRAM
 * strategy, and start/stop the LLM + image runtimes. The shared state lives
 * in the localMode slice so both surfaces stay in sync.
 */
function LocalMistralRsFields() {
  const { t } = useTranslation('settings');
  const { t: tLocal } = useTranslation('local_mode');
  const lm = useStore((s) => s.localMode);
  // Poll runtime status so the pills + the toWireConfig port lookup reflect
  // the current sidecar state. While the local-mistralrs panel is mounted
  // we always poll - the user is actively configuring it, so the original
  // `enabled` gate would be a confusing extra hoop.
  useLocalRuntimeStatus(true);

  const [startStatus, setStartStatus] = useState<RuntimeActionStatus>('idle');
  const [stopStatus, setStopStatus] = useState<RuntimeActionStatus>('idle');
  const startResetRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const stopResetRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearStartReset = useCallback(() => {
    if (startResetRef.current !== null) {
      clearTimeout(startResetRef.current);
      startResetRef.current = null;
    }
  }, []);

  const clearStopReset = useCallback(() => {
    if (stopResetRef.current !== null) {
      clearTimeout(stopResetRef.current);
      stopResetRef.current = null;
    }
  }, []);

  useEffect(
    () => () => {
      clearStartReset();
      clearStopReset();
    },
    [clearStartReset, clearStopReset],
  );

  const handleStart = useCallback(async () => {
    clearStartReset();
    setStartStatus('pending');
    try {
      const res = await fetch('/local/runtime/start', { method: 'POST' });
      if (!res.ok) throw new Error(`http ${res.status}`);
      setStartStatus('idle');
    } catch {
      setStartStatus('error');
      startResetRef.current = setTimeout(() => {
        setStartStatus('idle');
        startResetRef.current = null;
      }, RUNTIME_RESET_DELAY_MS);
    }
  }, [clearStartReset]);

  const handleStop = useCallback(async () => {
    clearStopReset();
    setStopStatus('pending');
    try {
      const res = await fetch('/local/runtime/stop', { method: 'POST' });
      if (!res.ok) throw new Error(`http ${res.status}`);
      setStopStatus('idle');
    } catch {
      setStopStatus('error');
      stopResetRef.current = setTimeout(() => {
        setStopStatus('idle');
        stopResetRef.current = null;
      }, RUNTIME_RESET_DELAY_MS);
    }
  }, [clearStopReset]);

  return (
    <div className={styles.localFields}>
      <div className={styles.localHint}>{t('local_runtime_hint')}</div>

      <h3 className={styles.localHeading}>{tLocal('llm_models')}</h3>
      {LOCAL_LLMS.map((m) => (
        <LocalModelCard key={m.id} entry={m} isLlm />
      ))}

      <h3 className={styles.localHeading}>{tLocal('image_model')}</h3>
      <LocalModelCard
        entry={{ id: 'sdxl_turbo', name: 'SDXL-Turbo (fp16)', size: 7e9 }}
        isLlm={false}
      />

      <Field label={tLocal('vram_strategy')}>
        {({ id }) => (
          <select
            id={id}
            value={lm.vramStrategy}
            onChange={(e) => lm.setVramStrategy(e.target.value as VramStrategy)}
            className={styles.fullWidth}
          >
            <option value="auto-swap">{tLocal('strategy_auto_swap')}</option>
            <option value="keep-both-loaded">{tLocal('strategy_keep_both')}</option>
            <option value="disable-image-gen">{tLocal('strategy_disable_image')}</option>
          </select>
        )}
      </Field>

      <h3 className={styles.localHeading}>{t('local_runtime_section')}</h3>
      <div className={styles.localRuntimeRow}>
        <button
          type="button"
          disabled={startStatus === 'pending'}
          data-status={startStatus}
          onClick={() => {
            void handleStart();
          }}
        >
          {startStatus === 'pending' ? tLocal('runtime_starting') : tLocal('start_runtimes')}
        </button>
        {startStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_start_error')}
          </span>
        )}
        <button
          type="button"
          disabled={stopStatus === 'pending'}
          data-status={stopStatus}
          onClick={() => {
            void handleStop();
          }}
        >
          {stopStatus === 'pending' ? tLocal('runtime_stopping') : tLocal('stop_runtimes')}
        </button>
        {stopStatus === 'error' && (
          <span role="alert" className={styles.localErrorChip}>
            {tLocal('runtime_stop_error')}
          </span>
        )}
        <RuntimeStatusPill label={tLocal('runtime_pill_llm')} state={lm.runtime.llm} />
        <RuntimeStatusPill label={tLocal('runtime_pill_image')} state={lm.runtime.image} />
      </div>
      {lm.runtime.llm.state !== 'ready' && (
        <div className={styles.localHint}>{t('local_runtime_not_ready')}</div>
      )}
    </div>
  );
}

/**
 * Inline ModelDownloadCard binding that wires a manifest entry to the
 * download hook + the localMode slice. Mirrors the helper inside
 * LocalModeModal so both surfaces share the same selection semantics.
 */
function LocalModelCard({
  entry,
  isLlm,
}: {
  entry: { id: ModelId; name: string; size: number; vram?: number; warn?: string };
  isLlm: boolean;
}) {
  const lm = useStore((s) => s.localMode);
  const dl = useModelDownload(entry.id);
  return (
    <ModelDownloadCard
      modelId={entry.id}
      displayName={entry.name}
      sizeBytes={entry.size}
      vramBytes={entry.vram}
      vramWarning={entry.warn}
      state={lm.downloads[entry.id]}
      active={isLlm && lm.selectedLlm === entry.id}
      onSelect={() => isLlm && lm.selectModel(entry.id)}
      onDownload={() => {
        void dl.start();
      }}
      onDelete={() => {
        void dl.cancel();
      }}
    />
  );
}

function LanguageSelect({
  id,
  value,
  onChange,
}: {
  id: string;
  value: 'en' | 'ru';
  onChange: (lang: 'en' | 'ru') => void;
}) {
  const { t } = useTranslation('settings');
  return (
    <select
      id={id}
      value={value}
      onChange={(e) => onChange(e.target.value as 'en' | 'ru')}
      className={styles.fullWidth}
    >
      <option value="en">{t('lang_en')}</option>
      <option value="ru">{t('lang_ru')}</option>
    </select>
  );
}

interface ModelDraft {
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string;
}

function ModelTab({
  draft,
  onChange,
}: {
  draft: ModelDraft;
  onChange: (patch: Partial<ModelDraft>) => void;
}) {
  const { t } = useTranslation('settings');
  const sceneTransitionsEnabled = useStore((s) => s.settings.sceneTransitionsEnabled);
  const setSceneTransitionsEnabled = useStore((s) => s.settings.setSceneTransitionsEnabled);
  return (
    <>
      <Field label={t('system_prompt_label')} helper={t('system_prompt_helper')}>
        {(p) => (
          <textarea
            {...p}
            value={draft.systemPrompt}
            onChange={(e) => onChange({ systemPrompt: e.target.value })}
            placeholder={t('system_prompt_placeholder')}
            rows={6}
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={`${t('temperature_label')}: ${draft.temperature.toFixed(1)}`}>
        {(p) => (
          <input
            {...p}
            type="range"
            min={0}
            max={2}
            step={0.1}
            value={draft.temperature}
            onChange={(e) => onChange({ temperature: Number(e.target.value) })}
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={t('replicate_key_label')} helper={t('replicate_key_helper')}>
        {(p) => (
          <input
            {...p}
            type="password"
            autoComplete="off"
            value={draft.replicateApiKey}
            onChange={(e) => onChange({ replicateApiKey: e.target.value })}
            placeholder="r8_..."
            className={styles.fullWidth}
          />
        )}
      </Field>
      <Field label={t('scene_transitions_label')} helper={t('scene_transitions_helper')}>
        {(p) => (
          <label className={styles.toggleRow}>
            <input
              {...p}
              type="checkbox"
              checked={sceneTransitionsEnabled}
              onChange={(e) => setSceneTransitionsEnabled(e.target.checked)}
            />
            <span>{t('scene_transitions_toggle')}</span>
          </label>
        )}
      </Field>
    </>
  );
}

// ---- Draft state shape + validation ------------------------------------

interface DraftState {
  anthropic: AnthropicDraft;
  'openai-compat': OpenaiCompatDraft;
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string;
}

interface DraftErrors {
  apiKey?: string;
  baseUrl?: string;
  model?: string;
}

function initialDrafts(slice: {
  providers: { anthropic: AnthropicConfig | null; 'openai-compat': OpenaiCompatConfig | null };
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string | null;
}): DraftState {
  const a = slice.providers.anthropic;
  const o = slice.providers['openai-compat'];
  return {
    anthropic: {
      apiKey: a?.apiKey ?? '',
      model: a?.model ?? DEFAULT_ANTHROPIC_MODEL,
    },
    'openai-compat': {
      baseUrl: o?.baseUrl ?? '',
      apiKey: o?.apiKey ?? '',
      model: o?.model ?? '',
    },
    uiLanguage: slice.uiLanguage,
    narrationLanguage: slice.narrationLanguage,
    systemPrompt: slice.systemPrompt,
    temperature: slice.temperature,
    replicateApiKey: slice.replicateApiKey ?? '',
  };
}

type BuildResult = { ok: true; config: ProviderConfig } | { ok: false; errors: DraftErrors };

function buildConfig(
  kind: ProviderKind,
  drafts: DraftState,
  localSelectedLlm: ModelId,
): BuildResult {
  switch (kind) {
    case 'anthropic': {
      const errors: DraftErrors = {};
      const apiKey = parseApiKey(drafts.anthropic.apiKey);
      if (apiKey === null) errors.apiKey = 'required';
      const model = drafts.anthropic.model.trim();
      if (model.length === 0) errors.model = 'required';
      if (apiKey === null || model.length === 0) return { ok: false, errors };
      return { ok: true, config: { kind: 'anthropic', apiKey, model } };
    }
    case 'openai-compat': {
      const errors: DraftErrors = {};
      const baseUrl = parseBaseUrl(drafts['openai-compat'].baseUrl);
      if (baseUrl === null) errors.baseUrl = 'invalid_url';
      const apiKey = parseApiKey(drafts['openai-compat'].apiKey);
      if (apiKey === null) errors.apiKey = 'required';
      const model = drafts['openai-compat'].model.trim();
      if (model.length === 0) errors.model = 'required';
      if (baseUrl === null || apiKey === null || model.length === 0) return { ok: false, errors };
      return { ok: true, config: { kind: 'openai-compat', baseUrl, apiKey, model } };
    }
    case 'local-mistralrs': {
      // The selected ModelId lives in the localMode slice. We stash it in
      // `modelPath` and let the API boundary translate it to the wire shape
      // (model_id + port) when posting to /settings.
      const config: LocalMistralRsConfig = {
        kind: 'local-mistralrs',
        modelPath: localSelectedLlm,
        contextWindow: DEFAULT_LOCAL_CONTEXT_WINDOW,
      };
      return { ok: true, config };
    }
    default:
      return assertNeverProvider(kind);
  }
}
