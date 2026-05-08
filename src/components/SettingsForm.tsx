import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  type AnthropicConfig,
  assertNeverProvider,
  DEFAULT_ANTHROPIC_MODEL,
  type OpenaiCompatConfig,
  type ProviderConfig,
  type ProviderKind,
  parseApiKey,
  parseBaseUrl,
} from '../state/providers';
import { useStore } from '../state/useStore';
import { Field } from '../ui/Field';
import styles from './SettingsForm.module.css';

const PROVIDER_KINDS: readonly ProviderKind[] = ['anthropic', 'openai-compat'];

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
}

/**
 * Multi-provider Settings form.
 *
 * The form keeps each provider kind's draft state independent so the user
 * can flip between Anthropic and OpenAI-compat without losing what they've
 * typed. `local-mistralrs` is hidden from the UI in M1.5 (M4 lights it up);
 * the type union still covers it so a future variant is one branch in this
 * switch + one new sub-form component.
 *
 * M3 adds a tab bar at the top: the Provider tab keeps the existing fields
 * (provider picker + per-kind sub-form + language selects), and the Model
 * tab exposes the agent-loop knobs (system prompt, temperature, Replicate
 * API key) wired to `POST /agent-settings`.
 */
export function SettingsForm({ onSubmit, formId }: SettingsFormProps) {
  const { t } = useTranslation('settings');
  const slice = useStore((s) => s.settings);

  const [activeTab, setActiveTab] = useState<Tab>('provider');
  const [activeKind, setActiveKind] = useState<ProviderKind>(slice.activeProvider);
  const [drafts, setDrafts] = useState<DraftState>(() => initialDrafts(slice));
  const [submitting, setSubmitting] = useState(false);
  const [errors, setErrors] = useState<DraftErrors>({});

  useEffect(() => {
    setActiveKind(slice.activeProvider);
    setDrafts(initialDrafts(slice));
  }, [slice]);

  const onSave = async () => {
    const result = buildConfig(activeKind, drafts);
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
          aria-selected={activeTab === 'provider'}
          className={`${styles.tab} ${activeTab === 'provider' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('provider')}
        >
          {t('tab_provider')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === 'model'}
          className={`${styles.tab} ${activeTab === 'model' ? styles.tabActive : ''}`}
          onClick={() => setActiveTab('model')}
        >
          {t('tab_model')}
        </button>
      </div>

      {activeTab === 'provider' && (
        <>
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
                        | 'provider_openai_compat',
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

          {activeKind === 'local-mistralrs' && <LocalMistralRsPlaceholder />}

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
        </>
      )}

      {activeTab === 'model' && (
        <ModelTab
          draft={{
            systemPrompt: drafts.systemPrompt,
            temperature: drafts.temperature,
            replicateApiKey: drafts.replicateApiKey,
          }}
          onChange={(patch) => setDrafts((prev) => ({ ...prev, ...patch }))}
        />
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

function LocalMistralRsPlaceholder() {
  const { t } = useTranslation('settings');
  return <div className={styles.placeholder}>{t('local_coming_soon')}</div>;
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

function buildConfig(kind: ProviderKind, drafts: DraftState): BuildResult {
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
    case 'local-mistralrs':
      // Not implemented in M1.5.
      return { ok: false, errors: {} };
    default:
      return assertNeverProvider(kind);
  }
}
