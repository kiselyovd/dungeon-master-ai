/**
 * HeroStep - final step of the onboarding flow (hero creation / completion).
 *
 * Placeholder implementation for E1. Full hero creation UI (class picker,
 * finalize + completeOnboarding call) will be fleshed out in E6. For now
 * renders the step name and Back/Finish controls so the state machine is
 * testable end-to-end.
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';
import type { PresetId } from '../presets';

export interface HeroStepProps {
  titleId: string;
  preset: PresetId; // fleshed out in E6 - hero creation form
  onBack: () => void;
  onNext: () => void;
}

export function HeroStep({ titleId, onBack, onNext }: HeroStepProps) {
  const { t } = useTranslation('onboarding');

  return (
    <>
      <div className="dm-onboarding-tag">{t('step_create_hero')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step_hero_title')}
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
          className="dm-onboarding-btn dm-onboarding-btn-primary dm-onboarding-btn-lg"
          onClick={onNext}
        >
          {t('begin')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}
