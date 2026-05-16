import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import type { AssistField, FlagContext } from '../../../api/characterAssist';
import { useStore } from '../../../state/useStore';
import { PersonaTab } from '../PersonaTab';

const ACOLYTE_BG = {
  id: 'acolyte',
  name_en: 'Acolyte',
  name_ru: 'Послушник',
  skill_proficiencies: [],
  tool_proficiencies: [],
  language_proficiencies: {},
  starting_equipment: [],
  starting_gold: 15,
  feature: { name_en: '', name_ru: '', description: '' },
  suggested_characteristics: {
    personality_traits: ['I idolize a particular hero of my faith.', 'I see omens in every event.'],
    ideals: [],
    bonds: ['I would die to recover an ancient relic of my faith.'],
    flaws: [],
  },
};

const compendium = {
  classes: [],
  races: [],
  backgrounds: [ACOLYTE_BG],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

const generateFieldSpy = vi.fn<(field: AssistField, flagContext?: FlagContext) => Promise<void>>();
vi.mock('../../../hooks/useCharacterAssist', () => ({
  useCharacterAssist: () => ({
    generateField: generateFieldSpy,
    surpriseMe: vi.fn(async () => {}),
    runTestChat: vi.fn(async () => ''),
    cancel: vi.fn(),
  }),
}));

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
  generateFieldSpy.mockClear();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('PersonaTab personality-flag sparkles', () => {
  it('renders a sparkle button per slot (6 total)', () => {
    render(<PersonaTab compendium={compendium} />);
    const section = screen.getByText('Personality Flags').closest('div');
    expect(section).not.toBeNull();
    const sparkles = within(section as HTMLElement).getAllByRole('button', {
      name: /generate this personality flag/i,
    });
    expect(sparkles).toHaveLength(6);
  });

  it('clicking sparkle on bg-trait calls generateField with the right flag context', async () => {
    useStore.getState().charCreation.setDraftField('backgroundId', 'acolyte');
    render(<PersonaTab compendium={compendium} />);
    const section = screen.getByText('Personality Flags').closest('div') as HTMLElement;
    const sparkles = within(section).getAllByRole('button', {
      name: /generate this personality flag/i,
    });
    await userEvent.click(sparkles[0] as HTMLElement);
    expect(generateFieldSpy).toHaveBeenCalledTimes(1);
    const args = generateFieldSpy.mock.calls[0];
    expect(args?.[0]).toBe('personality_flag');
    expect(args?.[1]).toEqual({
      slotId: 'bg-trait',
      source: 'background',
      sourceLabel: 'Acolyte',
      pool: ['I idolize a particular hero of my faith.', 'I see omens in every event.'],
    });
  });

  it('sparkle is disabled while isAssisting is true', () => {
    useStore.getState().charCreation.setIsAssisting(true);
    render(<PersonaTab compendium={compendium} />);
    const section = screen.getByText('Personality Flags').closest('div') as HTMLElement;
    const sparkles = within(section).getAllByRole('button', {
      name: /generate this personality flag/i,
    });
    for (const btn of sparkles) {
      expect(btn).toBeDisabled();
    }
  });
});
