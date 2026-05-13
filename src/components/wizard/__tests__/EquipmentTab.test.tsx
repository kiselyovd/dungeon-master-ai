import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { EquipmentTab } from '../EquipmentTab';

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
      starting_gold: 100,
    },
  ],
  races: [],
  backgrounds: [],
  spells: [],
  equipment: {
    weapons: [
      {
        id: 'longsword',
        name_en: 'Longsword',
        name_ru: 'Длинный меч',
        category: 'martial',
        cost: { gp: 15 },
        damage: { dice: '1d8', type: 'slashing' },
        weight_lb: 3,
        properties: [],
        range_ft: {},
        source_url: '',
        srd_section: '',
      },
    ],
    armor: [],
    adventuring_gear: [],
  },
  feats: [],
  weapon_properties: [],
} as never;

beforeEach(() => {
  const s = useStore.getState().charCreation;
  s.resetDraft();
  s.setDraftField('classId', 'fighter');
});

describe('EquipmentTab', () => {
  it('renders mode toggle', () => {
    render(<EquipmentTab compendium={compendium} />);
    expect(screen.getByRole('radio', { name: /package/i })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /gold/i })).toBeInTheDocument();
  });

  it('switching to Gold mode seeds goldRemaining from class', async () => {
    render(<EquipmentTab compendium={compendium} />);
    await userEvent.click(screen.getByRole('radio', { name: /gold/i }));
    expect(useStore.getState().charCreation.goldRemaining).toBe(100);
  });

  it('clicking add adds item to inventory and deducts gold', async () => {
    render(<EquipmentTab compendium={compendium} />);
    await userEvent.click(screen.getByRole('radio', { name: /gold/i }));
    const addBtn = await screen.findByRole('button', { name: /add to inventory/i });
    await userEvent.click(addBtn);
    expect(useStore.getState().charCreation.equipmentInventory).toHaveLength(1);
    expect(useStore.getState().charCreation.goldRemaining).toBeCloseTo(85, 2);
  });
});
