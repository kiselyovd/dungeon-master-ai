import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { KEY_ART } from '../assets/livingTabletop';
import type { OnboardingEntry } from '../state/onboarding';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';
import { Icons } from '../ui/Icons';
import { LanguageSegmentedControl } from '../ui/LanguageSegmentedControl';

interface OnboardingProps {
  onJoinRoom?: () => void;
  onComplete?: (entry: OnboardingEntry) => void;
  hideLanguage?: boolean;
}

/** First choice records navigation intent only. Model/media setup and hero
 * creation belong to their real campaign or room context. */
export function Onboarding({ onComplete, onJoinRoom, hideLanguage = false }: OnboardingProps) {
  const { t } = useTranslation('onboarding');
  const { t: tc } = useTranslation('common');
  const titleId = useId();
  const language = useStore((state) => state.settings.uiLanguage);
  const setLanguage = useStore((state) => state.settings.setUiLanguage);
  const choose = useStore((state) => state.onboarding.chooseEntry);
  return (
    <div className="dm-onboarding">
      <div
        className="dm-onboarding-card dm-entry-card"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="dm-onboarding-header">
          <Icons.Crest size={26} />
          {!hideLanguage && (
            <LanguageSegmentedControl
              value={language}
              onChange={setLanguage}
              ariaLabel={tc('language')}
            />
          )}
        </div>
        <div className="dm-onboarding-hero" data-art-direction="living-tabletop">
          <img src={KEY_ART.onboarding} alt="" data-testid="onboarding-hero-art" />
        </div>
        <h1 id={titleId} className="dm-onboarding-title">
          {t('entry_title')}
        </h1>
        <div className="dm-entry-actions">
          <Button
            variant="primary"
            onClick={() => {
              choose('adventure');
              onComplete?.('adventure');
            }}
          >
            <Icons.BookOpen size={18} />
            {t('entry_adventure')}
          </Button>
          <Button
            onClick={() => {
              choose('guest');
              onJoinRoom?.();
            }}
            disabled={!onJoinRoom}
          >
            <Icons.User size={18} />
            {t('entry_join')}
          </Button>
        </div>
      </div>
    </div>
  );
}
