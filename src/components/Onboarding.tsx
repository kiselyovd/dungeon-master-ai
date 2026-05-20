import { Fragment, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Language } from '../state/settings';
import { useStore } from '../state/useStore';
import type { PresetId, Step } from './onboarding/presets';
import { computeSteps, DEFAULT_PRESET } from './onboarding/presets';
import { ChatStep } from './onboarding/steps/ChatStep';
import { HeroStep } from './onboarding/steps/HeroStep';
import { ImageStep } from './onboarding/steps/ImageStep';
import { PresetStep } from './onboarding/steps/PresetStep';
import { VideoStep } from './onboarding/steps/VideoStep';
import { WelcomeStep } from './onboarding/steps/WelcomeStep';

/**
 * First-run Onboarding wizard (P2.12 E1). Six steps: Welcome, Preset,
 * Chat, Image (optional), Video (optional), Hero.
 *
 * Mounted by `App.tsx` over the entire UI when `state.onboarding.completed`
 * is `false`. After the Hero step finalizes we flip the flag and dismount.
 * CharacterWizard is then mounted by App.tsx (condition: !pc.heroClass) to
 * let the user create their hero.
 *
 * The step sequence is computed from `computeSteps(preset)` so optional steps
 * (image, video) are inserted or removed based on the user's preset choice.
 * Before a preset is chosen the default sequence is used for the stepper.
 *
 * Public prop contract: only `onComplete?: () => void` - unchanged from the
 * prior 2-step implementation so App.tsx does not need any edits.
 */

interface OnboardingProps {
  /** Optional callback invoked after finalization; primarily for tests. */
  onComplete?: () => void;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const { t } = useTranslation('onboarding');
  const { t: tCommon } = useTranslation('common');
  const titleId = useId();

  const [preset, setPreset] = useState<PresetId>(DEFAULT_PRESET);
  const [stepIndex, setStepIndex] = useState(0);

  const steps = computeSteps(preset);
  const currentStep = steps[stepIndex];
  const totalSteps = steps.length;

  const completeOnboarding = useStore((s) => s.onboarding.complete);
  const uiLanguage = useStore((s) => s.settings.uiLanguage);
  const setUiLanguage = useStore((s) => s.settings.setUiLanguage);

  const next = (): void => {
    if (stepIndex < steps.length - 1) {
      setStepIndex((i) => i + 1);
    } else {
      finalize();
    }
  };

  const back = (): void => {
    if (stepIndex > 0) {
      setStepIndex((i) => i - 1);
    }
  };

  const finalize = (): void => {
    completeOnboarding();
    onComplete?.();
  };

  // Step label lookup: each step maps to an i18n key.
  // Record<Step, ...> ensures a compile error if a new Step variant is added
  // without updating this map.
  const stepLabelKey: Record<Step, string> = {
    welcome: 'step_welcome',
    preset: 'step_pick_preset',
    chat: 'step_connect_ai',
    image: 'step_image',
    video: 'step_video',
    hero: 'step_create_hero',
  };

  const stepLabels = steps.map((s) => t(stepLabelKey[s]));

  // Counter text uses the step_counter key with {{current}} and {{total}}.
  const counterText = t('step_counter', { current: stepIndex + 1, total: totalSteps });

  return (
    <div className="dm-onboarding">
      <div
        className="dm-onboarding-card dm-vignette"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-label={t('dialog_label')}
      >
        <LanguagePicker
          value={uiLanguage}
          onChange={setUiLanguage}
          ariaLabel={tCommon('language')}
        />

        <div className="dm-onboarding-counter" aria-live="polite">
          {counterText}
        </div>

        <ol className="dm-onboarding-steps" aria-label={t('stepper_label')}>
          {stepLabels.map((label, i) => (
            <Fragment key={label}>
              <li
                className={`dm-onboarding-step${i === stepIndex ? ' is-active' : ''}${i < stepIndex ? ' is-done' : ''}`}
                aria-current={i === stepIndex ? 'step' : undefined}
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

        {currentStep === 'welcome' && <WelcomeStep titleId={titleId} onNext={next} />}
        {currentStep === 'preset' && (
          <PresetStep
            titleId={titleId}
            preset={preset}
            onPresetChange={(p) => {
              setPreset(p);
              // Re-clamp the step index when preset changes shrink the sequence.
              setStepIndex((i) => {
                const newSteps = computeSteps(p);
                return Math.min(i, newSteps.length - 1);
              });
            }}
            onBack={back}
            onNext={next}
          />
        )}
        {currentStep === 'chat' && (
          <ChatStep titleId={titleId} preset={preset} onBack={back} onNext={next} />
        )}
        {currentStep === 'image' && (
          <ImageStep titleId={titleId} preset={preset} onBack={back} onNext={next} onSkip={next} />
        )}
        {currentStep === 'video' && (
          <VideoStep titleId={titleId} preset={preset} onBack={back} onNext={next} onSkip={next} />
        )}
        {currentStep === 'hero' && (
          <HeroStep titleId={titleId} preset={preset} onBack={back} onNext={next} />
        )}
      </div>
    </div>
  );
}

// ---- LanguagePicker ----------------------------------------------------

interface LanguagePickerProps {
  value: Language;
  onChange: (lang: Language) => void;
  ariaLabel: string;
}

/**
 * Two-button toggle (EN / RU) sitting in the top-right of the onboarding card.
 * Visible on all steps so a brand-new user can flip language before
 * onboarding completes (Settings is reachable only afterwards). We deliberately
 * use the bare locale codes - flag emojis are politically charged for RU users.
 */
function LanguagePicker({ value, onChange, ariaLabel }: LanguagePickerProps) {
  const langs: readonly { code: Language; label: string }[] = [
    { code: 'en', label: 'EN' },
    { code: 'ru', label: 'RU' },
  ];
  return (
    // biome-ignore lint/a11y/useSemanticElements: <fieldset> would force a default border + legend layout that fights the pill-toggle styling; role="group" matches the WAI-ARIA Toolbar/Group pattern for inline controls
    <div className="dm-onboarding-lang" role="group" aria-label={ariaLabel}>
      {langs.map((l) => (
        <button
          key={l.code}
          type="button"
          className={`dm-onboarding-lang-btn${value === l.code ? ' is-active' : ''}`}
          aria-pressed={value === l.code}
          onClick={() => onChange(l.code)}
        >
          {l.label}
        </button>
      ))}
    </div>
  );
}
