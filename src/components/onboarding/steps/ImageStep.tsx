/**
 * ImageStep - optional step in the onboarding flow (image provider config).
 *
 * Placeholder implementation for E1. Skipped when preset is 'text-only'.
 * Full image provider UI will be fleshed out in E4. For now renders the step
 * name and Back/Next/Skip controls so the state machine is testable.
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';
import type { PresetId } from '../presets';

export interface ImageStepProps {
  titleId: string;
  preset: PresetId; // fleshed out in E4 - image provider form
  onBack: () => void;
  onNext: () => void;
  onSkip: () => void;
}

export function ImageStep({ titleId, onBack, onNext, onSkip }: ImageStepProps) {
  const { t } = useTranslation('onboarding');

  return (
    <>
      <div className="dm-onboarding-tag">{t('step_image')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step_image')}
      </h1>
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
        <button type="button" className="dm-onboarding-skip" onClick={onSkip}>
          {t('skip')}
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
