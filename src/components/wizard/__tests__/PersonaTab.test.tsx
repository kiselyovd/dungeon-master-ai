import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { PersonaTab } from '../PersonaTab';

const ACOLYTE_BG = {
  id: 'acolyte',
  name_en: 'Acolyte',
  name_ru: 'Послушник',
  skill_proficiencies: ['insight', 'religion'],
  tool_proficiencies: [],
  language_proficiencies: {},
  starting_equipment: [],
  starting_gold: 15,
  feature: { name_en: 'Shelter', name_ru: 'Приют', description: '' },
  suggested_characteristics: {
    personality_traits: [
      'I idolize a particular hero of my faith.',
      'I find common ground between fierce enemies.',
      'I see omens in every event.',
      'Nothing can shake my optimistic attitude.',
      'I quote sacred texts in almost every situation.',
      'I am tolerant of other faiths.',
      'I enjoy fine food and high society.',
      'I have little experience in the outside world.',
    ],
    ideals: [],
    bonds: [
      'I would die to recover an ancient relic of my faith.',
      'I will get revenge on the corrupt temple hierarchy.',
      'I owe my life to the priest who took me in.',
      'Everything I do is for the common people.',
      'I will do anything to protect my temple.',
      'I seek to preserve a sacred text.',
    ],
    flaws: [],
  },
};

const DWARF_RACE = {
  id: 'dwarf',
  name_en: 'Dwarf',
  name_ru: 'Дварф',
  ability_score_increases: { con: 2 },
  age: { mature_at: 50, max_lifespan: 350 },
  alignment_tendency:
    'Most dwarves are lawful, believing firmly in the benefits of a well-ordered society. They tend toward good, with a strong sense of fair play and a belief that everyone deserves to share in the benefits of a just order.',
  size: 'Medium',
  speed: 25,
  languages: ['Common', 'Dwarvish'],
  proficiencies: { skills: [], weapons: [], tools: [], saves: [] },
  senses: { darkvision_ft: 60 },
  traits: [
    {
      id: 'darkvision',
      name_en: 'Darkvision',
      name_ru: 'Тёмное зрение',
      mechanical_description: '',
    },
    {
      id: 'dwarven-resilience',
      name_en: 'Dwarven Resilience',
      name_ru: 'Дварфская устойчивость',
      mechanical_description: '',
    },
    {
      id: 'dwarven-combat-training',
      name_en: 'Dwarven Combat Training',
      name_ru: 'Дварфская боевая подготовка',
      mechanical_description: '',
    },
    {
      id: 'tool-proficiency',
      name_en: 'Tool Proficiency',
      name_ru: 'Владение инструментами',
      mechanical_description: '',
    },
    {
      id: 'stonecunning',
      name_en: 'Stonecunning',
      name_ru: 'Знание камня',
      mechanical_description: '',
    },
  ],
  subraces: [
    {
      id: 'hill-dwarf',
      name_en: 'Hill Dwarf',
      name_ru: 'Холмовой дварф',
      additional_asi: { wis: 1 },
      additional_traits: [
        {
          id: 'dwarven-toughness',
          name_en: 'Dwarven Toughness',
          name_ru: 'Дварфская выносливость',
          mechanical_description: '',
        },
      ],
    },
  ],
  source_url: '',
  srd_section: '',
};

const compendium = {
  classes: [],
  races: [DWARF_RACE],
  backgrounds: [ACOLYTE_BG],
  spells: [],
  equipment: { weapons: [], armor: [], adventuring_gear: [] },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
});

describe('PersonaTab', () => {
  it('renders name input + alignment grid + 4 textareas (regression)', () => {
    render(<PersonaTab compendium={compendium} />);
    expect(screen.getByLabelText('Name')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'LG' })).toBeInTheDocument();
    expect(screen.getByLabelText(/ideals/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/backstory/i)).toBeInTheDocument();
  });

  it('typing in name updates draft (regression)', async () => {
    render(<PersonaTab compendium={compendium} />);
    await userEvent.type(screen.getByLabelText('Name'), 'Astarion');
    expect(useStore.getState().charCreation.name).toBe('Astarion');
  });

  it('clicking alignment sets draft.alignment (regression)', async () => {
    render(<PersonaTab compendium={compendium} />);
    await userEvent.click(screen.getByRole('button', { name: 'CG' }));
    expect(useStore.getState().charCreation.alignment).toBe('CG');
  });

  describe('Personality Flags section', () => {
    it('renders all 6 flag rows with only Custom option when nothing is picked', () => {
      render(<PersonaTab compendium={compendium} />);
      expect(screen.getByText('Personality Flags')).toBeInTheDocument();
      const slotIds = [
        'bg-trait',
        'bg-bond',
        'align-trait',
        'align-quirk',
        'race-trait',
        'race-quirk',
      ];
      for (const slot of slotIds) {
        const select = document.getElementById(`dm-persona-flag-${slot}`) as HTMLSelectElement;
        expect(select).toBeTruthy();
        // Each select has at least 2 options: empty placeholder + Custom...
        // No preset pool entries when nothing is selected upstream.
        const presetOptions = Array.from(select.options).filter(
          (o) => o.value !== '' && o.value !== '__custom__',
        );
        expect(presetOptions).toHaveLength(0);
      }
    });

    it('bg-trait + bg-bond dropdowns populate when backgroundId=acolyte', () => {
      useStore.getState().charCreation.setDraftField('backgroundId', 'acolyte');
      render(<PersonaTab compendium={compendium} />);
      const traitSelect = document.getElementById('dm-persona-flag-bg-trait') as HTMLSelectElement;
      const traitPresets = Array.from(traitSelect.options).filter(
        (o) => o.value !== '' && o.value !== '__custom__',
      );
      expect(traitPresets.length).toBeGreaterThanOrEqual(4);
      // Cap is 6.
      expect(traitPresets.length).toBeLessThanOrEqual(6);
      expect(traitPresets[0]?.value).toContain('idolize');

      const bondSelect = document.getElementById('dm-persona-flag-bg-bond') as HTMLSelectElement;
      const bondPresets = Array.from(bondSelect.options).filter(
        (o) => o.value !== '' && o.value !== '__custom__',
      );
      expect(bondPresets.length).toBeGreaterThanOrEqual(4);
      expect(bondPresets.some((o) => o.value.includes('relic'))).toBe(true);
    });

    it('align-trait + align-quirk dropdowns populate when alignment=LG', () => {
      useStore.getState().charCreation.setDraftField('alignment', 'LG');
      render(<PersonaTab compendium={compendium} />);
      const traitSelect = document.getElementById(
        'dm-persona-flag-align-trait',
      ) as HTMLSelectElement;
      const traitValues = Array.from(traitSelect.options).map((o) => o.value);
      expect(traitValues).toContain('Honorable');
      expect(traitValues).toContain('Devout');

      const quirkSelect = document.getElementById(
        'dm-persona-flag-align-quirk',
      ) as HTMLSelectElement;
      const quirkValues = Array.from(quirkSelect.options).map((o) => o.value);
      expect(quirkValues).toContain('Self-righteous');
      expect(quirkValues).toContain('Inflexible');
    });

    it('race-trait + race-quirk dropdowns populate when raceId=dwarf', () => {
      useStore.getState().charCreation.setDraftField('raceId', 'dwarf');
      render(<PersonaTab compendium={compendium} />);
      const traitSelect = document.getElementById(
        'dm-persona-flag-race-trait',
      ) as HTMLSelectElement;
      const traitPresets = Array.from(traitSelect.options).filter(
        (o) => o.value !== '' && o.value !== '__custom__',
      );
      expect(traitPresets.length).toBeGreaterThanOrEqual(4);
      expect(traitPresets.some((o) => o.value === 'Darkvision')).toBe(true);

      const quirkSelect = document.getElementById(
        'dm-persona-flag-race-quirk',
      ) as HTMLSelectElement;
      const quirkPresets = Array.from(quirkSelect.options).filter(
        (o) => o.value !== '' && o.value !== '__custom__',
      );
      // dwarf has a multi-sentence alignment_tendency, so we expect >= 1 fragment.
      expect(quirkPresets.length).toBeGreaterThanOrEqual(1);
    });

    it('picking a preset value upserts a PersonalityFlag into the store', async () => {
      useStore.getState().charCreation.setDraftField('alignment', 'LG');
      render(<PersonaTab compendium={compendium} />);
      const traitSelect = document.getElementById(
        'dm-persona-flag-align-trait',
      ) as HTMLSelectElement;
      await userEvent.selectOptions(traitSelect, 'Honorable');
      const flags = useStore.getState().charCreation.personalityFlags;
      expect(flags).toEqual([{ slotId: 'align-trait', source: 'alignment', flag: 'Honorable' }]);
    });

    it('picking Custom and typing free text upserts the typed value', async () => {
      useStore.getState().charCreation.setDraftField('backgroundId', 'acolyte');
      render(<PersonaTab compendium={compendium} />);
      const select = document.getElementById('dm-persona-flag-bg-trait') as HTMLSelectElement;
      await userEvent.selectOptions(select, '__custom__');
      const input = screen.getByLabelText('Personality trait custom') as HTMLInputElement;
      await userEvent.type(input, 'Quirky bookworm');
      const flag = useStore
        .getState()
        .charCreation.personalityFlags.find((f) => f.slotId === 'bg-trait');
      expect(flag).toEqual({
        slotId: 'bg-trait',
        source: 'background',
        flag: 'Quirky bookworm',
      });
    });

    it('clearing the dropdown removes the flag entry from the store', async () => {
      useStore.getState().charCreation.setDraftField('alignment', 'LG');
      useStore
        .getState()
        .charCreation.setDraftField('personalityFlags', [
          { slotId: 'align-trait', source: 'alignment', flag: 'Honorable' },
        ]);
      render(<PersonaTab compendium={compendium} />);
      const select = document.getElementById('dm-persona-flag-align-trait') as HTMLSelectElement;
      await userEvent.selectOptions(select, '');
      const flags = useStore.getState().charCreation.personalityFlags;
      expect(flags).toEqual([]);
    });

    it('upserting overwrites the existing entry for the same slot (no duplicates)', async () => {
      useStore.getState().charCreation.setDraftField('alignment', 'LG');
      render(<PersonaTab compendium={compendium} />);
      const select = document.getElementById('dm-persona-flag-align-trait') as HTMLSelectElement;
      await userEvent.selectOptions(select, 'Honorable');
      await userEvent.selectOptions(select, 'Devout');
      const flags = useStore.getState().charCreation.personalityFlags;
      expect(flags).toHaveLength(1);
      expect(flags[0]).toEqual({
        slotId: 'align-trait',
        source: 'alignment',
        flag: 'Devout',
      });
    });
  });

  describe('section order (additive only, no reordering)', () => {
    it('Personality Flags section appears above Name input', () => {
      render(<PersonaTab compendium={compendium} />);
      const personaHeading = screen.getByRole('heading', { level: 2, name: 'Persona' });
      const root = personaHeading.closest('section') as HTMLElement;
      const flagsTitle = within(root).getByText('Personality Flags');
      const nameLabel = within(root).getByText('Name');
      // flagsTitle should appear before nameLabel in DOM order.
      expect(
        flagsTitle.compareDocumentPosition(nameLabel) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    });
  });
});
