import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { SkillsTab } from '../SkillsTab';

const compendium = {
  classes: [
    {
      id: 'fighter',
      name_en: 'Fighter',
      name_ru: 'Воин',
      hit_die: 10,
      primary_ability: ['STR'],
      saving_throw_proficiencies: ['str', 'con'],
      armor_proficiencies: [],
      weapon_proficiencies: [],
      tool_proficiencies: [],
      skill_proficiencies: {
        choose: 2,
        from: ['athletics', 'acrobatics', 'perception', 'intimidation'],
      },
      starting_equipment: {},
      level_1_features: {},
      spellcasting: null,
      subclasses: [],
      source_url: '',
    },
  ],
  backgrounds: [
    {
      id: 'acolyte',
      name_en: 'Acolyte',
      name_ru: 'Послушник',
      skill_proficiencies: ['insight'],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: {},
      feature: { name_en: '', name_ru: '', description: '' },
      suggested_characteristics: {},
    },
  ],
  races: [],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  const s = useStore.getState().charCreation;
  s.resetDraft();
  s.setDraftField('classId', 'fighter');
  s.setDraftField('backgroundId', 'acolyte');
});

describe('SkillsTab', () => {
  it('renders class skill options', () => {
    render(<SkillsTab compendium={compendium} />);
    expect(screen.getByText(/athletics/i)).toBeInTheDocument();
    expect(screen.getByText(/acrobatics/i)).toBeInTheDocument();
  });

  it('clicking adds skill to skillProfs up to quota', async () => {
    render(<SkillsTab compendium={compendium} />);
    await userEvent.click(screen.getByText(/athletics/i));
    expect(useStore.getState().charCreation.skillProfs).toContain('athletics');
  });

  it('shows pick-class-first notice when classId missing', () => {
    useStore.getState().charCreation.setDraftField('classId', null);
    render(<SkillsTab compendium={compendium} />);
    expect(screen.getByText(/pick.*class/i)).toBeInTheDocument();
  });
});
