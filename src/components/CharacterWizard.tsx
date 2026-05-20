import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type Compendium, fetchCompendium } from '../api/srd';
import type { CharacterDraft, WizardTab } from '../state/charCreation';
import { useStore } from '../state/useStore';
import { WizardLiveSheet } from './WizardLiveSheet';
import { AbilitiesTab } from './wizard/AbilitiesTab';
import { BackgroundTab } from './wizard/BackgroundTab';
import { ClassTab } from './wizard/ClassTab';
import { computeLiveSheet } from './wizard/computeLiveSheet';
import { EquipmentTab } from './wizard/EquipmentTab';
import { PersonaTab } from './wizard/PersonaTab';
import { PortraitTab } from './wizard/PortraitTab';
import { RaceTab } from './wizard/RaceTab';
import { ReviewTab } from './wizard/ReviewTab';
import { SkillsTab } from './wizard/SkillsTab';
import { SpellsTab } from './wizard/SpellsTab';

export type CharacterWizardMode = 'initial' | 'edit';

export interface CharacterWizardProps {
  mode: CharacterWizardMode;
  onClose?: () => void;
  onOpenImageSettings?: () => void;
  hidden?: boolean;
}

const TABS: readonly WizardTab[] = [
  'class',
  'race',
  'background',
  'abilities',
  'skills',
  'spells',
  'equipment',
  'persona',
  'portrait',
  'review',
];

function isTabValid(tab: WizardTab, draft: CharacterDraft): boolean {
  switch (tab) {
    case 'class':
      return draft.classId !== null;
    case 'race':
      return draft.raceId !== null;
    case 'background':
      return draft.backgroundId !== null;
    case 'abilities':
      return draft.abilityMethod !== null;
    case 'skills':
      return true;
    case 'spells':
      return true;
    case 'equipment':
      return true;
    case 'persona':
      return true;
    case 'portrait':
      return true;
    case 'review':
      return true;
  }
}

export function CharacterWizard({
  mode,
  onClose,
  onOpenImageSettings,
  hidden,
}: CharacterWizardProps) {
  const { t } = useTranslation('wizard');
  const activeTab = useStore((s) => s.charCreation.activeTab);
  const setActiveTab = useStore((s) => s.charCreation.setActiveTab);
  const draft = useStore((s) => s.charCreation);
  const [compendium, setCompendium] = useState<Compendium | null>(null);

  useEffect(() => {
    fetchCompendium()
      .then(setCompendium)
      .catch(() => {});
  }, []);

  const sheet = compendium ? computeLiveSheet(draft, compendium) : null;

  const currentIndex = TABS.indexOf(activeTab);
  const isFirst = currentIndex === 0;
  const isLast = activeTab === 'review';
  const tabValid = isTabValid(activeTab, draft);

  const handleBack = useCallback(() => {
    const prev = TABS[currentIndex - 1];
    if (prev !== undefined) setActiveTab(prev);
  }, [currentIndex, setActiveTab]);

  const handleNext = useCallback(() => {
    const next = TABS[currentIndex + 1];
    if (!isLast && tabValid && next !== undefined) setActiveTab(next);
  }, [currentIndex, isLast, tabValid, setActiveTab]);

  return (
    <div
      className="dm-wizard"
      style={hidden ? { display: 'none' } : undefined}
      {...(!hidden ? { role: 'dialog', 'aria-modal': true } : {})}
    >
      <div className="dm-wizard-strip" role="tablist">
        {TABS.map((tab, index) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={activeTab === tab}
            className={`dm-wizard-tab${activeTab === tab ? ' is-active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            <span className="dm-wizard-tab-num">{index + 1}</span>
            {t(`tab_${tab}_name`)}
          </button>
        ))}
      </div>
      <main className="dm-wizard-panel">
        {compendium && activeTab === 'class' && <ClassTab compendium={compendium} />}
        {compendium && activeTab === 'race' && <RaceTab compendium={compendium} />}
        {compendium && activeTab === 'background' && <BackgroundTab compendium={compendium} />}
        {compendium && activeTab === 'abilities' && <AbilitiesTab />}
        {compendium && activeTab === 'skills' && <SkillsTab compendium={compendium} />}
        {compendium && activeTab === 'spells' && <SpellsTab compendium={compendium} />}
        {compendium && activeTab === 'equipment' && <EquipmentTab compendium={compendium} />}
        {compendium && activeTab === 'persona' && <PersonaTab compendium={compendium} />}
        {compendium && activeTab === 'portrait' && (
          <PortraitTab {...(onOpenImageSettings ? { onOpenSettings: onOpenImageSettings } : {})} />
        )}
        {compendium && activeTab === 'review' && (
          <ReviewTab
            compendium={compendium}
            mode={mode}
            {...(onClose !== undefined ? { onClose } : {})}
          />
        )}
      </main>
      <footer className="dm-wizard-footer">
        <fieldset className="dm-wizard-footer-nav" aria-label={t('nav_footer_label')}>
          <button
            type="button"
            className="dm-wizard-nav-btn dm-wizard-nav-btn--back"
            onClick={handleBack}
            disabled={isFirst}
          >
            {t('nav_back')}
          </button>
          <span className="dm-wizard-step-label" aria-live="polite">
            {t('nav_step', { current: currentIndex + 1, total: TABS.length })}
          </span>
          {!isLast && (
            <button
              type="button"
              className="dm-wizard-nav-btn dm-wizard-nav-btn--next"
              onClick={handleNext}
              disabled={!tabValid}
              {...(!tabValid ? { title: t('nav_next_blocked') } : {})}
            >
              {t('nav_next')}
            </button>
          )}
        </fieldset>
      </footer>
      {sheet && <WizardLiveSheet sheet={sheet} />}
    </div>
  );
}
