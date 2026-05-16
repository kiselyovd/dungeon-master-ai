import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import '../../../i18n';
import type { Compendium } from '../../../api/srd';
import { useStore } from '../../../state/useStore';
import { ReviewTab } from '../ReviewTab';

const compendium: Compendium = {
  races: [],
  classes: [],
  backgrounds: [
    {
      id: 'acolyte',
      name_en: 'Acolyte',
      name_ru: '',
      skill_proficiencies: [],
      tool_proficiencies: [],
      language_proficiencies: {},
      starting_equipment: ['a holy symbol', 'vestments'],
      starting_gold: 15,
      feature: { name_en: '', name_ru: '', description: '' },
      suggested_characteristics: {},
    },
  ],
  spells: [],
  equipment: {
    weapons: [
      {
        id: 'longsword',
        name_en: 'Longsword',
        name_ru: '',
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
} as unknown as Compendium;

beforeEach(() => {
  useStore.getState().charCreation.resetDraft();
  useStore.getState().pc.setHeroClass(null);
});

describe('ReviewTab', () => {
  it('shows block warnings and disables Begin Adventure when draft incomplete', () => {
    render(<ReviewTab compendium={compendium} mode="initial" />);
    expect(screen.getByText(/pick a class first/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /begin/i })).toBeDisabled();
  });

  it('enables Begin Adventure when all blocking fields filled', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    render(<ReviewTab compendium={compendium} mode="initial" />);
    expect(screen.getByRole('button', { name: /begin/i })).toBeEnabled();
  });

  it('Begin Adventure writes draft into pc slice and resets', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'wizard');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('name', 'Gale');
    render(<ReviewTab compendium={compendium} mode="initial" />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    expect(pc.heroClass).toBe('wizard');
    expect(pc.name).toBe('Gale');
    expect(useStore.getState().charCreation.classId).toBeNull();
  });

  it('Package mode end-to-end: concrete + wildcard-resolved slot + bg items + gold row', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'package');
    s.setDraftField('equipmentSlots', [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'a',
        customName: 'longsword',
        fromBackground: false,
        resolvedItemIds: [],
      },
      {
        slotId: 'class-1',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: ['longsword'],
      },
    ]);
    render(<ReviewTab compendium={compendium} mode="initial" />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    const longsword = pc.inventory.find((it) => it.id === 'longsword');
    expect(longsword?.count).toBe(2);
    const gold = pc.inventory.find((it) => it.id === 'gold');
    expect(gold?.count).toBe(15);
    const vestments = pc.inventory.find((it) => /vestments/i.test(it.name));
    expect(vestments).toBeDefined();
  });

  it('Gold mode end-to-end: equipmentInventory + residual gold row', async () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'gold');
    s.setDraftField('equipmentInventory', [
      { id: 'longsword', name: 'Longsword', count: 1, icon: 'weapon' },
    ]);
    s.setDraftField('goldRemaining', 12.5);
    render(<ReviewTab compendium={compendium} mode="initial" />);
    await userEvent.click(screen.getByRole('button', { name: /begin/i }));
    const pc = useStore.getState().pc;
    const longsword = pc.inventory.find((it) => it.id === 'longsword');
    expect(longsword?.count).toBe(1);
    expect(longsword?.icon).toBe('sword');
    const gold = pc.inventory.find((it) => it.id === 'gold');
    expect(gold?.count).toBe(12);
  });

  it('shows unresolved wildcard warning when Package mode has unresolved chunks', () => {
    const s = useStore.getState().charCreation;
    s.setDraftField('classId', 'fighter');
    s.setDraftField('raceId', 'human');
    s.setDraftField('backgroundId', 'acolyte');
    s.setDraftField('abilityMethod', 'point_buy');
    s.setDraftField('equipmentMode', 'package');
    s.setDraftField('equipmentSlots', [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ]);
    render(<ReviewTab compendium={compendium} mode="initial" />);
    expect(screen.getByText(/some equipment choices are unresolved/i)).toBeInTheDocument();
  });
});
