/**
 * PresetStep - step 2 of 6 in the onboarding flow.
 *
 * Placeholder implementation for E1. Content (preset cards) will be fleshed
 * out in E2. For now renders the step name and Back/Next controls so the
 * state machine is testable end-to-end.
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';
import type { PresetId } from '../presets';

export interface PresetStepProps {
  titleId: string;
  preset: PresetId; // fleshed out in E2 - preset card selection
  onPresetChange: (preset: PresetId) => void; // fleshed out in E2
  onBack: () => void;
  onNext: () => void;
}

export function PresetStep({ titleId, onBack, onNext }: PresetStepProps) {
  const { t } = useTranslation('onboarding');

  return (
    <>
      <div className="dm-onboarding-tag">{t('step_pick_preset')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step_pick_preset')}
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
