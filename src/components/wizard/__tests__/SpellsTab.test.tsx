import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { SpellsTab } from '../SpellsTab';

const compendium = {
  classes: [
    {
      id: 'wizard',
      name_en: 'Wizard',
      name_ru: 'Маг',
      hit_die: 6,
      primary_ability: ['INT'],
      saving_throw_proficiencies: ['int', 'wis'],
      armor_proficiencies: [],
      weapon_proficiencies: [],
      tool_proficiencies: [],
      skill_proficiencies: {},
      starting_equipment: {},
      level_1_features: {},
      spellcasting: { cantrips_known: 3, spells_prepared: 6 },
      subclasses: [],
      source_url: '',
    },
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
      skill_proficiencies: {},
      starting_equipment: {},
      level_1_features: {},
      spellcasting: null,
      subclasses: [],
      source_url: '',
    },
  ],
  spells: [
    {
      id: 'fire-bolt',
      name_en: 'Fire Bolt',
      name_ru: 'Огненный снаряд',
      level: 0,
      school: 'evocation',
      casting_time: '1 action',
      range_ft: 120,
      components: { v: true, s: true },
      duration: 'instantaneous',
      ritual: false,
      concentration: false,
      classes: ['wizard'],
      description_en: '',
      description_ru: '',
      source_url: '',
      srd_section: '',
    },
    {
      id: 'magic-missile',
      name_en: 'Magic Missile',
      name_ru: 'Волшебная стрела',
      level: 1,
      school: 'evocation',
      casting_time: '1 action',
      range_ft: 120,
      components: { v: true, s: true },
      duration: 'instantaneous',
      ritual: false,
      concentration: false,
      classes: ['wizard'],
      description_en: '',
      description_ru: '',
      source_url: '',
      srd_section: '',
    },
  ],
  races: [],
  backgrounds: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('SpellsTab', () => {
  it('shows not-caster notice for non-spellcaster class', () => {
    useStore.getState().charCreation.setDraftField('classId', 'fighter');
    render(<SpellsTab compendium={compendium} />);
    expect(screen.getByText(/does not cast/i)).toBeInTheDocument();
  });

  it('renders cantrip and level-1 sections for caster', () => {
    useStore.getState().charCreation.setDraftField('classId', 'wizard');
    render(<SpellsTab compendium={compendium} />);
    expect(screen.getByText(/fire bolt/i)).toBeInTheDocument();
    expect(screen.getByText(/magic missile/i)).toBeInTheDocument();
  });

  it('clicking a cantrip adds it to draft.spells.cantrips', async () => {
    useStore.getState().charCreation.setDraftField('classId', 'wizard');
    render(<SpellsTab compendium={compendium} />);
    await userEvent.click(screen.getByText(/fire bolt/i));
    expect(useStore.getState().charCreation.spells.cantrips).toContain('fire-bolt');
  });
});
