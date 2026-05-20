/**
 * HeroStep - final step of the onboarding flow (E6).
 *
 * Renders 4 hero class cards (fighter / rogue / wizard / cleric). Clicking a
 * card calls `applyPreset` then `onNext`, which triggers the state machine's
 * `finalize()` (since hero is the last step). A "Build from scratch" button
 * at the bottom calls `onExitToWizard` then `onNext` without applying a preset.
 *
 * Card preview stats (race, HP, AC) are sourced directly from PRESETS in
 * pc.ts so they are always in sync with the real preset values.
 */

import { useTranslation } from 'react-i18next';
import { HERO_PORTRAIT } from '../../../lib/heroPortraits';
import { type HeroClassId, PRESETS } from '../../../state/pc';
import { useStore } from '../../../state/useStore';
import { Icons } from '../../../ui/Icons';

export interface HeroStepProps {
  titleId: string;
  onBack: () => void;
  onNext: () => void;
  onExitToWizard?: () => void;
}

const HERO_CLASSES: readonly HeroClassId[] = ['fighter', 'rogue', 'wizard', 'cleric'];

/** Short equipment summary shown on the card (top 3 non-gold items). */
function equipmentSummary(classId: HeroClassId): string {
  return (
    PRESETS[classId].inventory
      .filter((it) => it.icon !== 'coin')
      .slice(0, 3)
      // Shortens "Potion of Healing" -> "Potion" to fit the card summary.
      // Note: any future preset item whose name contains " of Healing" will also be shortened.
      .map((it) => it.name.replace(' of Healing', ''))
      .join(', ')
  );
}

export function HeroStep({ titleId, onBack, onNext, onExitToWizard }: HeroStepProps) {
  const { t } = useTranslation('onboarding');
  const applyPreset = useStore((s) => s.pc.applyPreset);

  function handleClassClick(classId: HeroClassId) {
    applyPreset(classId);
    onNext();
  }

  function handleBuildFromScratch() {
    onExitToWizard?.();
    onNext();
  }

  return (
    <>
      <div className="dm-onboarding-tag">{t('hero_step_tag')}</div>
      <h1 id={titleId} className="dm-onboarding-title">
        {t('step_hero_title')}
      </h1>

      <div className="dm-hero-cards">
        {HERO_CLASSES.map((classId) => {
          const preset = PRESETS[classId];
          const portrait = HERO_PORTRAIT[classId];
          const nameKey = `class_${classId}_name` as const;
          return (
            <button
              key={classId}
              type="button"
              className="dm-hero-card"
              onClick={() => handleClassClick(classId)}
            >
              <div className="dm-hero-card-portrait" aria-hidden="true">
                {portrait ? <img src={portrait} alt="" /> : <Icons.User size={32} />}
              </div>
              <div className="dm-hero-card-body">
                <span className="dm-hero-card-name">{t(nameKey)}</span>
                <span className="dm-hero-card-race">{preset.race}</span>
                <div className="dm-hero-card-stats">
                  <span className="dm-hero-card-stat">
                    <span className="dm-hero-card-stat-label">HP</span>
                    <span className="dm-hero-card-stat-value">{preset.hp}</span>
                  </span>
                  <span className="dm-hero-card-stat">
                    <span className="dm-hero-card-stat-label">AC</span>
                    <span className="dm-hero-card-stat-value">{preset.ac}</span>
                  </span>
                </div>
                <span className="dm-hero-card-equipment">{equipmentSummary(classId)}</span>
              </div>
            </button>
          );
        })}
      </div>

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
          className="dm-onboarding-btn dm-onboarding-btn-secondary"
          onClick={handleBuildFromScratch}
        >
          {t('hero_build_from_scratch')}
        </button>
      </div>
    </>
  );
}
