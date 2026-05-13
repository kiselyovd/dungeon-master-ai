import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type Compendium, fetchCompendium } from '../api/srd';
import type { WizardTab } from '../state/charCreation';
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

export function CharacterWizard({ mode, onClose }: CharacterWizardProps) {
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

  return (
    <div className="dm-wizard" role="dialog" aria-modal="true">
      <div className="dm-wizard-strip" role="tablist">
        {TABS.map((tab) => (
          <button
            key={tab}
            type="button"
            role="tab"
            aria-selected={activeTab === tab}
            className={`dm-wizard-tab${activeTab === tab ? ' is-active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
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
        {compendium && activeTab === 'portrait' && <PortraitTab />}
        {compendium && activeTab === 'review' && (
          <ReviewTab
            compendium={compendium}
            mode={mode}
            {...(onClose !== undefined ? { onClose } : {})}
          />
        )}
      </main>
      {sheet && <WizardLiveSheet sheet={sheet} />}
    </div>
  );
}
