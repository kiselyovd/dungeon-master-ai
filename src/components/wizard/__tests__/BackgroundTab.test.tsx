import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { BackgroundTab } from '../BackgroundTab';

const compendium = {
  backgrounds: [
    {
      id: 'acolyte',
      name_en: 'Acolyte',
      name_ru: 'Послушник',
      skill_proficiencies: ['insight', 'religion'],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: {},
      feature: { name_en: 'Shelter of the Faithful', name_ru: 'Sanc', description: '' },
      suggested_characteristics: {},
    },
  ],
  classes: [],
  races: [],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('BackgroundTab', () => {
  it('renders background cards', () => {
    render(<BackgroundTab compendium={compendium} />);
    expect(screen.getByText('Acolyte')).toBeInTheDocument();
  });

  it('clicking a card sets backgroundId', async () => {
    render(<BackgroundTab compendium={compendium} />);
    await userEvent.click(screen.getByText('Acolyte'));
    expect(useStore.getState().charCreation.backgroundId).toBe('acolyte');
  });

  it('shows SRD limit note', () => {
    render(<BackgroundTab compendium={compendium} />);
    expect(screen.getByText(/SRD 5.1/)).toBeInTheDocument();
  });
});
