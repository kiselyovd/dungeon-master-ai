/**
 * ChatStep - step 3 of 6 in the onboarding flow (chat provider config).
 *
 * Placeholder implementation for E1. Full provider form (Anthropic / OpenAI
 * compat / Local) will be fleshed out in E3. For now renders the step name
 * and Back/Next controls so the state machine is testable end-to-end.
 */

import { useTranslation } from 'react-i18next';
import { Icons } from '../../../ui/Icons';
import type { PresetId } from '../presets';

export interface ChatStepProps {
  titleId: string;
  preset: PresetId; // fleshed out in E3 - drives provider form variant
  onBack: () => void;
  onNext: () => void;
}

export function ChatStep({ titleId, onBack, onNext }: ChatStepProps) {
  const { t } = useTranslation('onboarding');

  return (
    <>
      <div className="dm-onboarding-tag">{t('step_connect_ai')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step_chat_title')}
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
