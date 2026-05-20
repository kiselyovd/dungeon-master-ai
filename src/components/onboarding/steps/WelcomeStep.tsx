/**
 * WelcomeStep - step 1 of 6 in the onboarding flow. Renders the welcome
 * title, flavour description, and the primary Next button (no Back on step 1).
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';

export interface WelcomeStepProps {
  titleId: string;
  onNext: () => void;
}

export function WelcomeStep({ titleId, onNext }: WelcomeStepProps) {
  const { t } = useTranslation('onboarding');

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
          {t('next')}
          <Icons.ChevronRight size={14} />
        </button>
      </div>
    </>
  );
}
