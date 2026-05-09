import { Fragment, type ReactNode, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { postSettings } from '../api/providers';
import {
  DEFAULT_ANTHROPIC_MODEL,
  DEFAULT_LOCAL_CONTEXT_WINDOW,
  parseApiKey,
  parseBaseUrl,
} from '../state/providers';
import { useStore } from '../state/useStore';
import { Icons } from '../ui/Icons';

/**
 * First-run Onboarding wizard (P2.12). Three steps: Welcome, Connect AI,
 * Create hero. Mounted by `App.tsx` over the entire UI when
 * `state.onboarding.completed` is `false`; once the user finishes step 3
 * we flip the flag, persist the chosen provider config + hero class, and
 * dismount.
 *
 * Provider sub-forms keep things light: Anthropic and OpenAI-compat get
 * inline validation (the same `parseApiKey` / `parseBaseUrl` helpers the
 * Settings form uses), and Local renders an explainer pointing at the
 * Settings -> Provider tab. The aspirational "Model file picker" from
 * the design jsx is deliberately not replicated here - the embedded
 * runtime needs more than a path string anyway, so onboarding stays the
 * fast-path while Settings owns the full configuration surface.
 *
 * Step 3 always saves a class (defaults to "fighter") because the rest
 * of the app relies on a non-null hero from this point on. Skipping
 * counts as accepting the default.
 */

type Step = 0 | 1 | 2;

type ProviderChoice = 'anthropic' | 'openai-compat' | 'local-mistralrs';

type HeroClassId = 'fighter' | 'wizard' | 'rogue' | 'cleric';

interface OnboardingProps {
  /** Optional callback invoked after persistence; primarily for tests. */
  onComplete?: () => void;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const { t } = useTranslation('onboarding');
  const titleId = useId();

  const [step, setStep] = useState<Step>(0);
  const [providerChoice, setProviderChoice] = useState<ProviderChoice>('anthropic');
  const [providerErrors, setProviderErrors] = useState<ProviderErrors>({});
  const [anthropicKey, setAnthropicKey] = useState('');
  const [openaiBaseUrl, setOpenaiBaseUrl] = useState('https://api.openai.com/v1');
  const [openaiKey, setOpenaiKey] = useState('');
  const [openaiModel, setOpenaiModel] = useState('');
  const [heroClass, setHeroClass] = useState<HeroClassId>('fighter');

  const setActiveProvider = useStore((s) => s.settings.setActiveProvider);
  const setProviderConfig = useStore((s) => s.settings.setProviderConfig);
  const setHeroClassInStore = useStore((s) => s.pc.setHeroClass);
  const completeOnboarding = useStore((s) => s.onboarding.complete);

  const stepLabels: readonly string[] = [
    t('step_welcome'),
    t('step_connect_ai'),
    t('step_create_hero'),
  ];

  const advanceFromStep2 = (): void => {
    const errors = validateProviderChoice(providerChoice, {
      anthropicKey,
      openaiBaseUrl,
      openaiKey,
      openaiModel,
    });
    if (Object.keys(errors).length > 0) {
      setProviderErrors(errors);
      return;
    }
    setProviderErrors({});
    setStep(2);
  };

  const finalize = async (): Promise<void> => {
    // Persist the provider choice into the settings slice. We always have a
    // valid config at this point because step 2 validation gates step 3.
    const apiKey = parseApiKey(anthropicKey);
    const openaiUrl = parseBaseUrl(openaiBaseUrl);
    const openaiApiKey = parseApiKey(openaiKey);

    if (providerChoice === 'anthropic' && apiKey !== null) {
      setProviderConfig({
        kind: 'anthropic',
        apiKey,
        model: DEFAULT_ANTHROPIC_MODEL,
      });
    } else if (
      providerChoice === 'openai-compat' &&
      openaiUrl !== null &&
      openaiApiKey !== null &&
      openaiModel.trim().length > 0
    ) {
      setProviderConfig({
        kind: 'openai-compat',
        baseUrl: openaiUrl,
        apiKey: openaiApiKey,
        model: openaiModel.trim(),
      });
    } else if (providerChoice === 'local-mistralrs') {
      // Lightweight stub - the real runtime configuration happens in the
      // Settings -> Provider tab. We still persist the kind so downstream
      // code knows which branch the user picked.
      setProviderConfig({
        kind: 'local-mistralrs',
        modelPath: 'qwen3_5_4b',
        contextWindow: DEFAULT_LOCAL_CONTEXT_WINDOW,
      });
    }
    setActiveProvider(providerChoice);
    setHeroClassInStore(heroClass);
    completeOnboarding();

    // Push the new provider to the backend so the chat path is wired up
    // before the user hits send. Errors here are non-fatal: the same
    // POST happens from the Settings modal, so a transient miss can be
    // recovered there. We do not block the modal close on it.
    if (providerChoice === 'anthropic' && apiKey !== null) {
      void postSettings({
        kind: 'anthropic',
        apiKey,
        model: DEFAULT_ANTHROPIC_MODEL,
      }).catch(() => {});
    } else if (
      providerChoice === 'openai-compat' &&
      openaiUrl !== null &&
      openaiApiKey !== null &&
      openaiModel.trim().length > 0
    ) {
      void postSettings({
        kind: 'openai-compat',
        baseUrl: openaiUrl,
        apiKey: openaiApiKey,
        model: openaiModel.trim(),
      }).catch(() => {});
    }

    onComplete?.();
  };

  return (
    <div className="dm-onboarding">
      <div
        className="dm-onboarding-card dm-vignette"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={t('dialog_label')}
      >
        <ol className="dm-onboarding-steps" aria-label={t('stepper_label')}>
          {stepLabels.map((label, i) => (
            <Fragment key={label}>
              <li
                className={`dm-onboarding-step${i === step ? ' is-active' : ''}${i < step ? ' is-done' : ''}`}
                aria-current={i === step ? 'step' : undefined}
              >
                <span className="dm-onboarding-step-dot" aria-hidden="true" />
                <span>{label}</span>
              </li>
              {i < stepLabels.length - 1 && (
                <span className="dm-onboarding-step-line" aria-hidden="true" />
              )}
            </Fragment>
          ))}
        </ol>

        {step === 0 && <Step1 titleId={titleId} t={t} onNext={() => setStep(1)} />}
        {step === 1 && (
          <Step2
            titleId={titleId}
            t={t}
            choice={providerChoice}
            onChoiceChange={(c) => {
              setProviderChoice(c);
              setProviderErrors({});
            }}
            anthropicKey={anthropicKey}
            onAnthropicKeyChange={setAnthropicKey}
            openaiBaseUrl={openaiBaseUrl}
            onOpenaiBaseUrlChange={setOpenaiBaseUrl}
            openaiKey={openaiKey}
            onOpenaiKeyChange={setOpenaiKey}
            openaiModel={openaiModel}
            onOpenaiModelChange={setOpenaiModel}
            errors={providerErrors}
            onBack={() => setStep(0)}
            onNext={advanceFromStep2}
          />
        )}
        {step === 2 && (
          <Step3
            titleId={titleId}
            t={t}
            heroClass={heroClass}
            onHeroClassChange={setHeroClass}
            onBack={() => setStep(1)}
            onSkip={() => {
              // Skip == accept the current default and finalize.
              void finalize();
            }}
            onBegin={() => {
              void finalize();
            }}
          />
        )}
      </div>
    </div>
  );
}

// ---- Step 1 ------------------------------------------------------------

interface Step1Props {
  titleId: string;
  t: (key: string) => string;
  onNext: () => void;
}

function Step1({ titleId, t, onNext }: Step1Props) {
  return (
    <>
      <div className="dm-onboarding-glyph-row">
        <div className="dm-app-mark-glyph dm-onboarding-glyph-lg">
          <Icons.D20 size={32} />
        </div>
      </div>
      <div className="dm-onboarding-tag">{t('step1_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step1_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('step1_desc')}</p>
      <div className="dm-onboarding-actions">
        <button
          type="button"
          className="dm-onboarding-btn dm-onboarding-btn-primary dm-onboarding-btn-lg"
          onClick={onNext}
        >
          <Icons.Sparkle size={14} />
          {t('step1_cta')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}

// ---- Step 2 ------------------------------------------------------------

interface Step2Props {
  titleId: string;
  t: (key: string) => string;
  choice: ProviderChoice;
  onChoiceChange: (choice: ProviderChoice) => void;
  anthropicKey: string;
  onAnthropicKeyChange: (value: string) => void;
  openaiBaseUrl: string;
  onOpenaiBaseUrlChange: (value: string) => void;
  openaiKey: string;
  onOpenaiKeyChange: (value: string) => void;
  openaiModel: string;
  onOpenaiModelChange: (value: string) => void;
  errors: ProviderErrors;
  onBack: () => void;
  onNext: () => void;
}

function Step2({
  titleId,
  t,
  choice,
  onChoiceChange,
  anthropicKey,
  onAnthropicKeyChange,
  openaiBaseUrl,
  onOpenaiBaseUrlChange,
  openaiKey,
  onOpenaiKeyChange,
  openaiModel,
  onOpenaiModelChange,
  errors,
  onBack,
  onNext,
}: Step2Props) {
  const anthropicKeyId = useId();
  const openaiBaseUrlId = useId();
  const openaiKeyId = useId();
  const openaiModelId = useId();

  return (
    <>
      <div className="dm-onboarding-tag">{t('step2_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step2_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('step2_desc')}</p>

      <div className="dm-provider-cards" role="radiogroup" aria-label={t('step2_title')}>
        <ProviderCardMini
          active={choice === 'anthropic'}
          onClick={() => onChoiceChange('anthropic')}
          icon={<Icons.Cloud size={18} />}
          title={t('provider_anthropic_title')}
          desc={t('provider_anthropic_desc')}
        />
        <ProviderCardMini
          active={choice === 'openai-compat'}
          onClick={() => onChoiceChange('openai-compat')}
          icon={<Icons.Server size={18} />}
          title={t('provider_openai_title')}
          desc={t('provider_openai_desc')}
        />
        <ProviderCardMini
          active={choice === 'local-mistralrs'}
          onClick={() => onChoiceChange('local-mistralrs')}
          icon={<Icons.Cpu size={18} />}
          title={t('provider_local_title')}
          desc={t('provider_local_desc')}
        />
      </div>

      {choice === 'anthropic' && (
        <div className="dm-onboarding-form">
          <FieldRow
            id={anthropicKeyId}
            label={t('anthropic_api_key_label')}
            hint={t('anthropic_api_key_hint')}
            error={errors.anthropicKey ? t(errors.anthropicKey) : undefined}
            required
          >
            <input
              id={anthropicKeyId}
              type="password"
              autoComplete="off"
              className="dm-onboarding-form-input"
              value={anthropicKey}
              onChange={(e) => onAnthropicKeyChange(e.target.value)}
              placeholder={t('anthropic_api_key_placeholder')}
              aria-invalid={errors.anthropicKey ? true : undefined}
            />
          </FieldRow>
        </div>
      )}

      {choice === 'openai-compat' && (
        <div className="dm-onboarding-form">
          <FieldRow
            id={openaiBaseUrlId}
            label={t('openai_base_url_label')}
            error={errors.openaiBaseUrl ? t(errors.openaiBaseUrl) : undefined}
            required
          >
            <input
              id={openaiBaseUrlId}
              type="url"
              className="dm-onboarding-form-input"
              value={openaiBaseUrl}
              onChange={(e) => onOpenaiBaseUrlChange(e.target.value)}
              placeholder={t('openai_base_url_placeholder')}
              aria-invalid={errors.openaiBaseUrl ? true : undefined}
            />
          </FieldRow>
          <FieldRow
            id={openaiKeyId}
            label={t('openai_api_key_label')}
            error={errors.openaiKey ? t(errors.openaiKey) : undefined}
            required
          >
            <input
              id={openaiKeyId}
              type="password"
              autoComplete="off"
              className="dm-onboarding-form-input"
              value={openaiKey}
              onChange={(e) => onOpenaiKeyChange(e.target.value)}
              placeholder={t('openai_api_key_placeholder')}
              aria-invalid={errors.openaiKey ? true : undefined}
            />
          </FieldRow>
          <FieldRow
            id={openaiModelId}
            label="Model"
            error={errors.openaiModel ? t(errors.openaiModel) : undefined}
            required
          >
            <input
              id={openaiModelId}
              type="text"
              className="dm-onboarding-form-input"
              value={openaiModel}
              onChange={(e) => onOpenaiModelChange(e.target.value)}
              placeholder="qwen3-1.7b"
              aria-invalid={errors.openaiModel ? true : undefined}
            />
          </FieldRow>
        </div>
      )}

      {choice === 'local-mistralrs' && (
        <div className="dm-onboarding-local-hint" role="note">
          {t('local_setup_hint')}
        </div>
      )}

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
          onClick={onNext}
        >
          {t('next')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}

// ---- Step 3 ------------------------------------------------------------

interface Step3Props {
  titleId: string;
  t: (key: string) => string;
  heroClass: HeroClassId;
  onHeroClassChange: (heroClass: HeroClassId) => void;
  onBack: () => void;
  onSkip: () => void;
  onBegin: () => void;
}

function Step3({ titleId, t, heroClass, onHeroClassChange, onBack, onSkip, onBegin }: Step3Props) {
  const classes: readonly {
    id: HeroClassId;
    icon: ReactNode;
  }[] = [
    { id: 'fighter', icon: <Icons.Sword size={22} /> },
    { id: 'wizard', icon: <Icons.Wand size={22} /> },
    { id: 'rogue', icon: <Icons.Bow size={22} /> },
    { id: 'cleric', icon: <Icons.Star size={22} /> },
  ];

  return (
    <>
      <div className="dm-onboarding-tag">{t('step3_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step3_title')}
      </h1>
      <p className="dm-onboarding-desc">{t('step3_desc')}</p>

      <div className="dm-class-grid" role="radiogroup" aria-label={t('step3_title')}>
        {classes.map((c) => (
          // biome-ignore lint/a11y/useSemanticElements: rich card content (icon + name + desc) needs a button surface; native <input type="radio"> cannot host this layout
          <button
            key={c.id}
            type="button"
            role="radio"
            aria-checked={heroClass === c.id}
            className={`dm-class-card${heroClass === c.id ? ' is-selected' : ''}`}
            onClick={() => onHeroClassChange(c.id)}
          >
            <div className="dm-class-card-icon">{c.icon}</div>
            <div className="dm-class-card-name">{t(`class_${c.id}_name`)}</div>
            <div className="dm-class-card-desc">{t(`class_${c.id}_desc`)}</div>
          </button>
        ))}
      </div>

      <div className="dm-onboarding-actions">
        <button type="button" className="dm-onboarding-skip" onClick={onSkip}>
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
          onClick={onBegin}
        >
          <Icons.Sparkle size={14} />
          {t('begin')}
        </button>
      </div>
    </>
  );
}

// ---- ProviderCardMini --------------------------------------------------

interface ProviderCardMiniProps {
  active: boolean;
  onClick: () => void;
  icon: ReactNode;
  title: string;
  desc: string;
}

function ProviderCardMini({ active, onClick, icon, title, desc }: ProviderCardMiniProps) {
  return (
    // biome-ignore lint/a11y/useSemanticElements: radio card hosts an icon + title + desc layout that <input type="radio"> cannot represent
    <button
      type="button"
      role="radio"
      aria-checked={active}
      className={`dm-provider-card${active ? ' is-active' : ''}`}
      onClick={onClick}
    >
      <span className="dm-provider-card-radio" aria-hidden="true">
        {active && <span />}
      </span>
      <span className="dm-provider-card-icon" aria-hidden="true">
        {icon}
      </span>
      <span className="dm-provider-card-body">
        <span className="dm-provider-card-title">{title}</span>
        <span className="dm-provider-card-desc">{desc}</span>
      </span>
    </button>
  );
}

// ---- Field row helper --------------------------------------------------

interface FieldRowProps {
  id: string;
  label: string;
  hint?: string | undefined;
  error?: string | undefined;
  required?: boolean | undefined;
  children: ReactNode;
}

function FieldRow({ id, label, hint, error, required, children }: FieldRowProps) {
  return (
    <div>
      <label htmlFor={id} className="dm-onboarding-tag" style={{ marginBottom: 6 }}>
        {label}
        {required && <span aria-hidden="true"> *</span>}
      </label>
      {children}
      {error ? (
        <div role="alert" className="dm-onboarding-form-error">
          {error}
        </div>
      ) : hint ? (
        <div className="dm-onboarding-form-hint">{hint}</div>
      ) : null}
    </div>
  );
}

// ---- Validation --------------------------------------------------------

interface ProviderErrors {
  anthropicKey?: 'validation_required';
  openaiBaseUrl?: 'validation_required' | 'validation_invalid_url';
  openaiKey?: 'validation_required';
  openaiModel?: 'validation_required';
}

interface Drafts {
  anthropicKey: string;
  openaiBaseUrl: string;
  openaiKey: string;
  openaiModel: string;
}

function validateProviderChoice(choice: ProviderChoice, drafts: Drafts): ProviderErrors {
  const errors: ProviderErrors = {};
  if (choice === 'anthropic') {
    if (parseApiKey(drafts.anthropicKey) === null) errors.anthropicKey = 'validation_required';
  } else if (choice === 'openai-compat') {
    if (drafts.openaiBaseUrl.trim().length === 0) {
      errors.openaiBaseUrl = 'validation_required';
    } else if (parseBaseUrl(drafts.openaiBaseUrl) === null) {
      errors.openaiBaseUrl = 'validation_invalid_url';
    }
    if (parseApiKey(drafts.openaiKey) === null) errors.openaiKey = 'validation_required';
    if (drafts.openaiModel.trim().length === 0) errors.openaiModel = 'validation_required';
  }
  // local-mistralrs: nothing to validate at the onboarding stage.
  return errors;
}
