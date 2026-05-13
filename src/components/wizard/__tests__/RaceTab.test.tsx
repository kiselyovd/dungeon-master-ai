import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { RaceTab } from '../RaceTab';

const compendium = {
  races: [
    {
      id: 'human',
      name_en: 'Human',
      name_ru: 'Человек',
      size: 'Medium',
      speed: 30,
      ability_score_increases: { str: 1 },
      age: { mature_at: 18, max_lifespan: 80 },
      languages: ['Common'],
      proficiencies: { skills: [], weapons: [], tools: [], saves: [] },
      senses: {},
      traits: [],
      subraces: [],
      source_url: '',
      srd_section: '',
    },
    {
      id: 'dwarf',
      name_en: 'Dwarf',
      name_ru: 'Дварф',
      size: 'Medium',
      speed: 25,
      ability_score_increases: { con: 2 },
      age: { mature_at: 50, max_lifespan: 350 },
      languages: ['Common', 'Dwarvish'],
      proficiencies: { skills: [], weapons: [], tools: [], saves: [] },
      senses: { darkvision_ft: 60 },
      traits: [],
      subraces: [
        {
          id: 'hill-dwarf',
          name_en: 'Hill Dwarf',
          name_ru: 'Холмовой Дварф',
          additional_asi: { wis: 1 },
          additional_traits: [],
        },
      ],
      source_url: '',
      srd_section: '',
    },
  ],
  classes: [],
  backgrounds: [],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('RaceTab', () => {
  it('renders race cards', () => {
    render(<RaceTab compendium={compendium} />);
    expect(screen.getByText('Human')).toBeInTheDocument();
    expect(screen.getByText('Dwarf')).toBeInTheDocument();
  });

  it('clicking a card sets raceId and clears subraceId', async () => {
    useStore.getState().charCreation.setDraftField('subraceId', 'stale');
    render(<RaceTab compendium={compendium} />);
    await userEvent.click(screen.getByText('Human'));
    expect(useStore.getState().charCreation.raceId).toBe('human');
    expect(useStore.getState().charCreation.subraceId).toBeNull();
  });

  it('subrace row appears when selected race has subraces', async () => {
    render(<RaceTab compendium={compendium} />);
    await userEvent.click(screen.getByText('Dwarf'));
    expect(await screen.findByText('Hill Dwarf')).toBeInTheDocument();
  });
});
