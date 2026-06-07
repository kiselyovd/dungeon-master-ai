import { type KeyboardEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { imageEntryForPreset, VIDEO_CATALOG } from '../api/providers-catalog';
import { useDiscoverProvider } from '../hooks/useDiscoverProvider';
import type { ModelId } from '../state/localMode';
import {
  assertNeverProvider,
  DEFAULT_LOCAL_CONTEXT_WINDOW,
  type LocalMistralRsConfig,
  OPENROUTER_BASE_URL,
  type OpenaiCompatConfig,
  type ProviderConfig,
  type ProviderKind,
  parseApiKey,
  parseBaseUrl,
} from '../state/providers';
import { useStore } from '../state/useStore';
import { Field } from '../ui/Field';
import { activeProviderCaps } from '../utils/capabilities';
import { isOssLicense } from '../utils/license';
import { ErrorBoundary } from './ErrorBoundary';
import { LicenseRestrictedBanner } from './LicenseRestrictedBanner';
import { ModelSelector } from './ModelSelector';
import styles from './SettingsForm.module.css';
import { LocalLlmTab } from './settings/LocalLlmTab';

const PROVIDER_KINDS: readonly ProviderKind[] = ['openai-compat', 'local-mistralrs'];

export type Tab = 'chat' | 'local-llm' | 'image' | 'video' | 'behavior';
const TAB_ORDER: readonly Tab[] = ['chat', 'local-llm', 'image', 'video', 'behavior'];

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
  /** Optional tab to select on mount; falls back to 'chat'. */
  initialTab?: Tab;
}

/**
 * Multi-provider Settings form.
 *
 * The form keeps each provider kind's draft state independent so the user
 * can flip between the OpenAI-compatible cloud provider and local-mistralrs
 * without losing what they've typed (native Anthropic was removed in M11 Batch
 * D.5). The local-mistralrs sub-form mirrors the
 * Ctrl+Shift+M LocalModeModal (model picker + VRAM strategy + runtime
 * controls) so the embedded provider can be configured from Settings.
 *
 * M3 added a tab bar at the top. D8 split it into five tabs: Chat keeps the
 * provider picker + per-kind sub-form, Local LLM hosts the standalone
 * local-mistralrs runtime + model UI, Image / Video host the media presets,
 * and Behavior exposes the agent-loop knobs (system prompt, temperature,
 * languages) wired to `POST /agent-settings`.
 */
export function SettingsForm({
  onSubmit,
  formId,
  onRequestCharacterRecreate,
  initialTab,
}: SettingsFormProps) {
  const { t } = useTranslation('settings');
  const slice = useStore((s) => s.settings);
  // Read the live ModelId from the localMode slice at submit time. We read
  // from getState() rather than subscribing because the selection only
  // matters at the moment Save fires, and a subscription here would force
  // every form rerender to also rebuild on unrelated download progress.
  const localModeSelection = useStore((s) => s.localMode.selectedLlm);

  const [activeTab, setActiveTab] = useState<Tab>(initialTab ?? 'chat');
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
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const idx = TAB_ORDER.indexOf(activeTab);
    const delta = e.key === 'ArrowLeft' ? -1 : 1;
    const next = TAB_ORDER[(idx + delta + TAB_ORDER.length) % TAB_ORDER.length] ?? 'chat';
    setActiveTab(next);
    requestAnimationFrame(() => {
      document.getElementById(`settings-tab-${next}`)?.focus();
    });
  };

  const onSave = async () => {
    const result = buildConfig(activeKind, drafts, localModeSelection);
    if (!result.ok) {
      // If the validation error lives in the Chat tab, surface it by
      // switching back to that tab so the inline message is visible.
      setActiveTab('chat');
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
          id="settings-tab-chat"
          aria-controls="settings-panel-chat"
          aria-selected={activeTab === 'chat'}
          tabIndex={activeTab === 'chat' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'chat' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('chat')}
        >
          {t('tab_chat')}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-tab-local-llm"
          aria-controls="settings-panel-local-llm"
          aria-selected={activeTab === 'local-llm'}
          tabIndex={activeTab === 'local-llm' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'local-llm' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('local-llm')}
        >
          {t('tab_local_llm')}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-tab-image"
          aria-controls="settings-panel-image"
          aria-selected={activeTab === 'image'}
          tabIndex={activeTab === 'image' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'image' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('image')}
        >
          {t('tab_image')}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-tab-video"
          aria-controls="settings-panel-video"
          aria-selected={activeTab === 'video'}
          tabIndex={activeTab === 'video' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'video' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('video')}
        >
          {t('tab_video')}
        </button>
        <button
          type="button"
          role="tab"
          id="settings-tab-behavior"
          aria-controls="settings-panel-behavior"
          aria-selected={activeTab === 'behavior'}
          tabIndex={activeTab === 'behavior' ? 0 : -1}
          onKeyDown={onTabKeyDown}
          className={`${styles.tab} ${activeTab === 'behavior' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('behavior')}
        >
          {t('tab_behavior')}
        </button>
      </div>

      {activeTab === 'chat' && (
        <div role="tabpanel" id="settings-panel-chat" aria-labelledby="settings-tab-chat">
          <ErrorBoundary level="section">
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
                          | 'provider_openai_compat'
                          | 'provider_local_mistralrs',
                      )}
                    </option>
                  ))}
                </select>
              )}
            </Field>

            {activeKind === 'openai-compat' && (
              <OpenaiCompatFields
                draft={drafts['openai-compat']}
                errors={errors}
                onChange={(d) => setDrafts((prev) => ({ ...prev, 'openai-compat': d }))}
              />
            )}

            {activeKind === 'local-mistralrs' && (
              <ErrorBoundary level="section">
                {/* Unified provider block: the local model + runtime controls
                    live inline here when the local provider is selected, so the
                    user configures the chosen provider in one place instead of
                    hunting for a separate tab. */}
                <LocalLlmTab />
              </ErrorBoundary>
            )}

            <ReasoningSection activeKind={activeKind} drafts={drafts} />

            {onRequestCharacterRecreate && (
              <section className={styles.characterSection}>
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
          </ErrorBoundary>
        </div>
      )}

      {activeTab === 'local-llm' && (
        <div role="tabpanel" id="settings-panel-local-llm" aria-labelledby="settings-tab-local-llm">
          <ErrorBoundary level="section">
            <LocalLlmTab />
          </ErrorBoundary>
        </div>
      )}

      {activeTab === 'image' && (
        <div role="tabpanel" id="settings-panel-image" aria-labelledby="settings-tab-image">
          <ErrorBoundary level="section">
            <ImageTab
              replicateApiKey={drafts.replicateApiKey}
              onReplicateApiKeyChange={(replicateApiKey) =>
                setDrafts((prev) => ({ ...prev, replicateApiKey }))
              }
            />
          </ErrorBoundary>
        </div>
      )}

      {activeTab === 'video' && (
        <div role="tabpanel" id="settings-panel-video" aria-labelledby="settings-tab-video">
          <ErrorBoundary level="section">
            <VideoTab />
          </ErrorBoundary>
        </div>
      )}

      {activeTab === 'behavior' && (
        <div role="tabpanel" id="settings-panel-behavior" aria-labelledby="settings-tab-behavior">
          <ErrorBoundary level="section">
            <ModelTab
              draft={{
                systemPrompt: drafts.systemPrompt,
                temperature: drafts.temperature,
              }}
              onChange={(patch) => setDrafts((prev) => ({ ...prev, ...patch }))}
            />
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
            <BehaviorExtras />
          </ErrorBoundary>
        </div>
      )}

      <input type="submit" hidden disabled={submitting} />
    </form>
  );
}

// ---- Sub-forms per provider --------------------------------------------

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
  const discovery = useDiscoverProvider({
    providerId: 'openai-compat',
    baseUrl: draft.baseUrl,
    apiKey: draft.apiKey,
  });
  return (
    <>
      <Field label={t('base_url_label')} helper={t('base_url_helper')} error={errors.baseUrl}>
        {(p) => (
          <input
            {...p}
            type="url"
            value={draft.baseUrl}
            onChange={(e) => onChange({ ...draft, baseUrl: e.target.value })}
            placeholder={OPENROUTER_BASE_URL}
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
        {() => (
          <ModelSelector
            value={draft.model}
            onChange={(model) => onChange({ ...draft, model })}
            models={discovery.models}
            status={discovery.status}
            error={discovery.error}
            onDiscover={discovery.discover}
            lastCachedAt={discovery.lastCachedAt}
            placeholder="qwen3-1.7b"
          />
        )}
      </Field>
    </>
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
  providers: { 'openai-compat': OpenaiCompatConfig | null };
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string | null;
}): DraftState {
  const o = slice.providers['openai-compat'];
  return {
    'openai-compat': {
      baseUrl: o?.baseUrl ?? OPENROUTER_BASE_URL,
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

// ---- M7-DM image / video / behavior-extras sub-forms --------------------

const IMAGE_PRESETS = [
  { id: 'fast' },
  { id: 'balanced' },
  { id: 'quality' },
  { id: 'quality-oss' },
  { id: 'cloud' },
] as const;

function ImageTab({
  replicateApiKey,
  onReplicateApiKeyChange,
}: {
  replicateApiKey: string;
  onReplicateApiKeyChange: (key: string) => void;
}) {
  const { t } = useTranslation('settings');
  const enabled = useStore((s) => s.settings.imageEnabled);
  const preset = useStore((s) => s.settings.imagePreset);
  const licenseRestricted = useStore((s) => s.settings.licenseRestrictedMode);
  const setImageEnabled = useStore((s) => s.settings.setImageEnabled);
  const setImagePreset = useStore((s) => s.settings.setImagePreset);

  const activeEntry = imageEntryForPreset(preset);
  const activeIsBlocked =
    licenseRestricted && activeEntry != null && !isOssLicense(activeEntry.license);
  const bannerPresetName = activeIsBlocked ? activeEntry?.display_name : null;

  return (
    <section className={styles.section}>
      <LicenseRestrictedBanner modality="image" activePresetName={bannerPresetName} />
      <label className={styles.checkboxRow}>
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setImageEnabled(e.target.checked)}
        />
        <span>{t('image_enable')}</span>
      </label>
      <fieldset disabled={!enabled}>
        <legend>{t('image_preset')}</legend>
        {IMAGE_PRESETS.map((p) => {
          const entry = imageEntryForPreset(p.id);
          const isBlocked = licenseRestricted && entry != null && !isOssLicense(entry.license);
          return (
            <label
              key={p.id}
              className={styles.radioRow}
              title={isBlocked ? t('license_restricted_tooltip') : undefined}
            >
              <input
                type="radio"
                name="image-preset"
                value={p.id}
                checked={preset === p.id}
                disabled={isBlocked}
                onChange={() => setImagePreset(p.id)}
              />
              <span>
                {t(`image_preset_${p.id.replace('-', '_')}` as const)}
                {isBlocked ? ` ${t('license_restricted_marker')}` : ''}
              </span>
            </label>
          );
        })}
      </fieldset>
      <Field label={t('replicate_key_label')} helper={t('replicate_key_helper')}>
        {(p) => (
          <input
            {...p}
            type="password"
            autoComplete="off"
            value={replicateApiKey}
            onChange={(e) => onReplicateApiKeyChange(e.target.value)}
            placeholder="r8_..."
            className={styles.fullWidth}
          />
        )}
      </Field>
    </section>
  );
}

const VIDEO_MODES = ['prerecorded', 'live', 'race'] as const;

function VideoTab() {
  const { t } = useTranslation('settings');
  const enabled = useStore((s) => s.settings.videoEnabled);
  const mode = useStore((s) => s.settings.videoMode);
  const licenseRestricted = useStore((s) => s.settings.licenseRestrictedMode);
  const setVideoEnabled = useStore((s) => s.settings.setVideoEnabled);
  const setVideoMode = useStore((s) => s.settings.setVideoMode);

  // Video v1 has exactly one provider (local-ltx-video). If its license is
  // non-OSS and restriction is on, disable the enable toggle and show a banner.
  const videoProvider = VIDEO_CATALOG[0];
  const videoProviderBlocked =
    licenseRestricted && videoProvider != null && !isOssLicense(videoProvider.license);
  const bannerVideoName = videoProviderBlocked ? videoProvider?.display_name : null;

  return (
    <section className={styles.section}>
      <LicenseRestrictedBanner modality="video" activePresetName={bannerVideoName} />
      <label
        className={styles.checkboxRow}
        title={videoProviderBlocked ? t('license_restricted_tooltip') : undefined}
      >
        <input
          type="checkbox"
          checked={enabled}
          disabled={videoProviderBlocked}
          onChange={(e) => setVideoEnabled(e.target.checked)}
        />
        <span>{t('video_enable')}</span>
      </label>
      <fieldset disabled={!enabled || videoProviderBlocked}>
        <legend>{t('video_mode')}</legend>
        {VIDEO_MODES.map((m) => (
          <label key={m} className={styles.radioRow}>
            <input
              type="radio"
              name="video-mode"
              value={m}
              checked={mode === m}
              onChange={() => setVideoMode(m)}
            />
            <span>{t(`video_mode_${m}` as const)}</span>
          </label>
        ))}
      </fieldset>
    </section>
  );
}

function ReasoningSection({
  activeKind,
  drafts,
}: {
  activeKind: ProviderKind;
  drafts: DraftState;
}) {
  const { t } = useTranslation('settings');
  const slice = useStore((s) => s.settings);

  const modelId = activeKind === 'openai-compat' ? drafts['openai-compat'].model : '';

  const caps = activeProviderCaps(activeKind, modelId);

  return (
    <fieldset disabled={!caps.reasoning} className={styles.fieldset}>
      <legend>{t('reasoning_section_label')}</legend>
      <label className={styles.checkboxRow}>
        <input
          type="checkbox"
          checked={slice.reasoningEnabled}
          onChange={(e) => slice.setReasoningEnabled(e.target.checked)}
        />
        <span>{t('reasoning_enable_label')}</span>
      </label>
      <label className={styles.checkboxRow}>
        {t('reasoning_budget_label')}
        <select
          value={slice.reasoningBudget}
          onChange={(e) => slice.setReasoningBudget(e.target.value as 'low' | 'medium' | 'high')}
          disabled={!slice.reasoningEnabled || !caps.reasoning}
        >
          <option value="low">{t('reasoning_budget_low')}</option>
          <option value="medium">{t('reasoning_budget_medium')}</option>
          <option value="high">{t('reasoning_budget_high')}</option>
        </select>
      </label>
      {!caps.reasoning && <p className={styles.hint}>{t('reasoning_unsupported_hint')}</p>}
    </fieldset>
  );
}

function BehaviorExtras() {
  const { t } = useTranslation('settings');
  const licenseRestrictedMode = useStore((s) => s.settings.licenseRestrictedMode);
  const setLicenseRestrictedMode = useStore((s) => s.settings.setLicenseRestrictedMode);
  const agentMaxRounds = useStore((s) => s.settings.agentMaxRounds);
  const setAgentMaxRounds = useStore((s) => s.settings.setAgentMaxRounds);
  return (
    <section className={styles.section}>
      <label className={styles.checkboxRow}>
        <input
          type="checkbox"
          checked={licenseRestrictedMode}
          onChange={(e) => setLicenseRestrictedMode(e.target.checked)}
        />
        <span>{t('license_restricted_mode')}</span>
      </label>
      <Field label={t('agent_max_rounds')}>
        {(p) => (
          <input
            {...p}
            type="number"
            min={1}
            max={32}
            value={agentMaxRounds}
            onChange={(e) => setAgentMaxRounds(Number(e.target.value) || 8)}
          />
        )}
      </Field>
    </section>
  );
}
