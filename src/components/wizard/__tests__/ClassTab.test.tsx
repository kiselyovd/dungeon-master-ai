import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { ClassTab } from '../ClassTab';

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
      skill_proficiencies: {},
      starting_equipment: {},
      level_1_features: {},
      spellcasting: null,
      subclasses: [],
      source_url: '',
    },
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
      spellcasting: {},
      subclasses: [],
      source_url: '',
    },
  ],
  races: [],
  backgrounds: [],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('ClassTab', () => {
  it('renders class cards', () => {
    render(<ClassTab compendium={compendium} />);
    expect(screen.getByText('Fighter')).toBeInTheDocument();
    expect(screen.getByText('Wizard')).toBeInTheDocument();
  });

  it('clicking a card sets classId in store', async () => {
    render(<ClassTab compendium={compendium} />);
    await userEvent.click(screen.getByText('Fighter'));
    expect(useStore.getState().charCreation.classId).toBe('fighter');
  });

  it('Surprise me button is rendered and enabled when not assisting', () => {
    render(<ClassTab compendium={compendium} />);
    expect(screen.getByRole('button', { name: /surprise/i })).toBeEnabled();
  });
});
