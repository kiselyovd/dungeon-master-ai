import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { EquipmentTab } from '../EquipmentTab';

const FIGHTER_STARTING = [
  { option_letter: 'a', items: ['chain mail'] },
  { option_letter: 'b', items: ['leather armor', 'longbow', '20 arrows'] },
  { option_letter: 'a', items: ['a martial weapon', 'shield'] },
  { option_letter: 'b', items: ['two martial weapons'] },
  { option_letter: 'a', items: ['light crossbow', '20 bolts'] },
  { option_letter: 'b', items: ['two handaxes'] },
  { option_letter: 'a', items: ["dungeoneer's pack"] },
  { option_letter: 'b', items: ["explorer's pack"] },
];

const ACOLYTE_BG = {
  id: 'acolyte',
  name_en: 'Acolyte',
  name_ru: 'Послушник',
  skill_proficiencies: ['insight', 'religion'],
  tool_proficiencies: [],
  language_proficiencies: {},
  starting_equipment: [
    'a holy symbol',
    'a prayer book',
    '5 sticks of incense',
    'vestments',
    'a set of common clothes',
    'a pouch containing 15 gp',
  ],
  starting_gold: 15,
  feature: { name_en: 'Shelter', name_ru: 'Приют', description: '' },
  suggested_characteristics: {},
};

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
      starting_equipment: FIGHTER_STARTING,
      level_1_features: {},
      spellcasting: null,
      subclasses: [],
      source_url: '',
      starting_gold: 100,
    },
  ],
  races: [],
  backgrounds: [ACOLYTE_BG],
  spells: [],
  equipment: {
    weapons: [
      {
        id: 'longsword',
        name_en: 'Longsword',
        name_ru: 'Длинный меч',
        category: 'martial_melee',
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

  describe('Package mode', () => {
    it('shows "select class first" hint when no class is picked', async () => {
      useStore.getState().charCreation.setDraftField('classId', null);
      render(<EquipmentTab compendium={compendium} />);
      await userEvent.click(screen.getByRole('radio', { name: /package/i }));
      expect(screen.getByText(/select a class first/i)).toBeInTheDocument();
      // No choice rows because there is no class
      expect(screen.queryByText(/choice 1/i)).not.toBeInTheDocument();
    });

    it('hydrates equipmentSlots on entering package mode with a class selected', async () => {
      render(<EquipmentTab compendium={compendium} />);
      await userEvent.click(screen.getByRole('radio', { name: /package/i }));
      // Fighter has 4 a/b choice groups, no fixed
      const slots = useStore.getState().charCreation.equipmentSlots;
      expect(slots).toHaveLength(4);
      expect(slots[0]?.slotId).toBe('class-0');
      expect(slots[0]?.itemId).toBe('a');
      expect(slots[0]?.customName).toBe('chain mail');
    });

    it('changing the dropdown swaps the chosen option and updates customName', async () => {
      render(<EquipmentTab compendium={compendium} />);
      await userEvent.click(screen.getByRole('radio', { name: /package/i }));
      // First select corresponds to chain mail vs leather armor/longbow/20 arrows
      const firstSelect = screen.getByLabelText(/choice 1/i) as HTMLSelectElement;
      await userEvent.selectOptions(firstSelect, 'b');
      const slots = useStore.getState().charCreation.equipmentSlots;
      expect(slots[0]?.itemId).toBe('b');
      expect(slots[0]?.customName).toBe('leather armor, longbow, 20 arrows');
    });

    it('renders the background items section in read-only form', async () => {
      useStore.getState().charCreation.setDraftField('backgroundId', 'acolyte');
      render(<EquipmentTab compendium={compendium} />);
      await userEvent.click(screen.getByRole('radio', { name: /package/i }));
      const heading = screen.getByRole('heading', { name: /background items/i });
      const section = heading.parentElement as HTMLElement;
      const list = within(section).getByRole('list');
      const items = within(list).getAllByRole('listitem');
      expect(items).toHaveLength(6);
      expect(items[0]?.textContent).toMatch(/holy symbol/i);
      expect(items[5]?.textContent).toMatch(/15 gp/i);
    });

    it('shows the empty-slot warning when a slot itemId is null', async () => {
      render(<EquipmentTab compendium={compendium} />);
      await userEvent.click(screen.getByRole('radio', { name: /package/i }));
      // Manually clear the first slot's itemId
      const cur = useStore.getState().charCreation.equipmentSlots;
      const next = cur.map((s, i) => (i === 0 ? { ...s, itemId: null } : s));
      useStore.getState().charCreation.setDraftField('equipmentSlots', next);
      // Re-render via state update propagates to the existing render automatically;
      // assert the warning is now visible.
      expect(await screen.findByText(/some slots are empty/i)).toBeInTheDocument();
    });

    describe('wildcard chooser', () => {
      it('renders an inner select for a wildcard slot', async () => {
        render(<EquipmentTab compendium={compendium} />);
        await userEvent.click(screen.getByRole('radio', { name: /package/i }));
        const slots = useStore
          .getState()
          .charCreation.equipmentSlots.map((s, i) =>
            i === 1 ? { ...s, itemId: 'wild', customName: 'any martial melee weapon' } : s,
          );
        useStore.getState().charCreation.setDraftField('equipmentSlots', slots);
        const innerSelect = await screen.findByLabelText(/choose a concrete weapon/i);
        expect(innerSelect).toBeInTheDocument();
      });

      it('picking inner option sets resolvedItemIds on the matching slot', async () => {
        render(<EquipmentTab compendium={compendium} />);
        await userEvent.click(screen.getByRole('radio', { name: /package/i }));
        const slots = useStore
          .getState()
          .charCreation.equipmentSlots.map((s, i) =>
            i === 1 ? { ...s, itemId: 'wild', customName: 'any martial melee weapon' } : s,
          );
        useStore.getState().charCreation.setDraftField('equipmentSlots', slots);
        const innerSelect = await screen.findByLabelText(/choose a concrete weapon/i);
        await userEvent.selectOptions(innerSelect, 'longsword');
        const slot = useStore
          .getState()
          .charCreation.equipmentSlots.find((s) => s.slotId === 'class-1');
        expect(slot?.resolvedItemIds).toEqual(['longsword']);
      });

      it('switching outer back to a concrete option clears resolvedItemIds', async () => {
        render(<EquipmentTab compendium={compendium} />);
        await userEvent.click(screen.getByRole('radio', { name: /package/i }));
        const slots = useStore.getState().charCreation.equipmentSlots.map((s, i) =>
          i === 1
            ? {
                ...s,
                itemId: 'wild',
                customName: 'any martial melee weapon',
                resolvedItemIds: ['longsword'],
              }
            : s,
        );
        useStore.getState().charCreation.setDraftField('equipmentSlots', slots);
        const outerSelects = screen.getAllByLabelText(/choice 2/i) as HTMLSelectElement[];
        const outer = outerSelects[0];
        if (!outer) throw new Error('expected choice-2 select');
        await userEvent.selectOptions(outer, 'b');
        const slot = useStore
          .getState()
          .charCreation.equipmentSlots.find((s) => s.slotId === 'class-1');
        expect(slot?.resolvedItemIds).toEqual([]);
      });
    });
  });
});
