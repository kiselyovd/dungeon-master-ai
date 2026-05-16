import { describe, expect, it } from 'vitest';
import type { Background, Compendium } from '../../../api/srd';
import type { EquipmentSlot } from '../../../state/charCreation';
import {
  computeGoldRows,
  filterCompendiumByWildcard,
  iconFor,
  lookupItemByName,
  mergeInventoryRows,
  parseEquipmentString,
  promoteIcon,
  readBackgroundStartingEquipment,
  resolveEquipmentSlots,
} from '../equipmentResolver';

const MINI_COMPENDIUM = {
  races: [],
  classes: [],
  backgrounds: [],
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
      {
        id: 'crossbow-light',
        name_en: 'Crossbow, light',
        name_ru: '',
        category: 'simple_ranged',
        cost: { gp: 25 },
        damage: { dice: '1d8', type: 'piercing' },
        weight_lb: 5,
        properties: [],
        range_ft: { normal: 80, long: 320 },
        source_url: '',
        srd_section: '',
      },
    ],
    armor: [
      {
        id: 'chain-mail',
        name_en: 'Chain Mail',
        name_ru: '',
        category: 'heavy',
        cost: { gp: 75 },
        ac_base: 16,
        stealth_disadvantage: true,
        weight_lb: 55,
        source_url: '',
        srd_section: '',
      },
      {
        id: 'shield',
        name_en: 'Shield',
        name_ru: '',
        category: 'shield',
        cost: { gp: 10 },
        ac_base: 2,
        stealth_disadvantage: false,
        weight_lb: 6,
        source_url: '',
        srd_section: '',
      },
    ],
    adventuring_gear: [
      {
        id: 'explorers-pack',
        name_en: "Explorer's Pack",
        name_ru: '',
        cost: { gp: 10 },
        weight_lb: 59,
      },
      {
        id: 'crossbow-bolts-20',
        name_en: 'Crossbow bolts (20)',
        name_ru: '',
        cost: { gp: 1 },
        weight_lb: 1.5,
      },
    ],
  },
  feats: [],
  weapon_properties: [],
} as unknown as Compendium;

describe('parseEquipmentString', () => {
  it('handles plain concrete item: "longsword"', () => {
    const r = parseEquipmentString('longsword');
    expect(r).toEqual({
      count: 1,
      nameKey: 'longsword',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('handles number-word + plural: "two handaxes"', () => {
    const r = parseEquipmentString('two handaxes');
    expect(r).toEqual({
      count: 2,
      nameKey: 'handaxe',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('handles digit count: "20 arrows" maps to bundle id', () => {
    const r = parseEquipmentString('20 arrows');
    expect(r).toEqual({
      count: 1,
      nameKey: 'arrows-20',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('handles digit count: "20 bolts" maps to bundle id', () => {
    const r = parseEquipmentString('20 bolts');
    expect(r).toEqual({
      count: 1,
      nameKey: 'crossbow-bolts-20',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('handles number-word: "five javelins"', () => {
    const r = parseEquipmentString('five javelins');
    expect(r).toEqual({
      count: 5,
      nameKey: 'javelin',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('detects wildcard: "any martial melee weapon"', () => {
    const r = parseEquipmentString('any martial melee weapon');
    expect(r).toEqual({
      count: 1,
      nameKey: 'martial melee weapon',
      isWildcard: true,
      ifProficient: false,
    });
  });

  it('detects ifProficient and strips suffix: "chain mail (if proficient)"', () => {
    const r = parseEquipmentString('chain mail (if proficient)');
    expect(r).toEqual({
      count: 1,
      nameKey: 'chain mail',
      isWildcard: false,
      ifProficient: true,
    });
  });

  it('strips leading article: "a holy symbol"', () => {
    const r = parseEquipmentString('a holy symbol');
    expect(r).toEqual({
      count: 1,
      nameKey: 'holy symbol',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('strips parenthetical: "a holy symbol (a gift to you...)"', () => {
    const r = parseEquipmentString('a holy symbol (a gift to you when you entered the priesthood)');
    expect(r).toEqual({
      count: 1,
      nameKey: 'holy symbol',
      isWildcard: false,
      ifProficient: false,
    });
  });

  it('keeps unrecognised plural for downstream lookup miss: "5 sticks of incense"', () => {
    const r = parseEquipmentString('5 sticks of incense');
    expect(r).toEqual({
      count: 5,
      nameKey: 'sticks of incense',
      isWildcard: false,
      ifProficient: false,
    });
  });
});

describe('lookupItemByName', () => {
  it('finds exact case-insensitive name match in weapons', () => {
    const r = lookupItemByName('longsword', MINI_COMPENDIUM);
    expect(r).toEqual({ id: 'longsword', name_en: 'Longsword', category: 'weapon' });
  });

  it('finds slug fallback for comma-name: "crossbow-light"', () => {
    const r = lookupItemByName('crossbow-light', MINI_COMPENDIUM);
    expect(r).toEqual({ id: 'crossbow-light', name_en: 'Crossbow, light', category: 'weapon' });
  });

  it('finds armor: "chain mail"', () => {
    const r = lookupItemByName('chain mail', MINI_COMPENDIUM);
    expect(r).toEqual({ id: 'chain-mail', name_en: 'Chain Mail', category: 'armor' });
  });

  it('classifies pack from id suffix: "explorers-pack"', () => {
    const r = lookupItemByName('explorers-pack', MINI_COMPENDIUM);
    expect(r).toEqual({ id: 'explorers-pack', name_en: "Explorer's Pack", category: 'pack' });
  });

  it('finds pack by name_en: "explorer\'s pack"', () => {
    const r = lookupItemByName("explorer's pack", MINI_COMPENDIUM);
    expect(r?.id).toBe('explorers-pack');
    expect(r?.category).toBe('pack');
  });

  it('returns null on miss', () => {
    const r = lookupItemByName('flugelhorn', MINI_COMPENDIUM);
    expect(r).toBeNull();
  });

  it('finds ammo bundle by id', () => {
    const r = lookupItemByName('crossbow-bolts-20', MINI_COMPENDIUM);
    expect(r).toEqual({
      id: 'crossbow-bolts-20',
      name_en: 'Crossbow bolts (20)',
      category: 'gear',
    });
  });
});

describe('iconFor', () => {
  it('shield armor -> shield', () => {
    expect(iconFor({ id: 'shield', name_en: 'Shield', category: 'armor' })).toBe('shield');
  });
  it('chain mail -> shield', () => {
    expect(iconFor({ id: 'chain-mail', name_en: 'Chain Mail', category: 'armor' })).toBe('shield');
  });
  it('crossbow weapon -> bow', () => {
    expect(iconFor({ id: 'crossbow-light', name_en: 'Crossbow, light', category: 'weapon' })).toBe(
      'bow',
    );
  });
  it('longsword weapon -> sword', () => {
    expect(iconFor({ id: 'longsword', name_en: 'Longsword', category: 'weapon' })).toBe('sword');
  });
  it("explorer's pack -> scroll", () => {
    expect(iconFor({ id: 'explorers-pack', name_en: "Explorer's Pack", category: 'pack' })).toBe(
      'scroll',
    );
  });
  it('gold row -> coin', () => {
    expect(iconFor({ id: 'gold', name_en: 'Gold pieces', category: 'gear' })).toBe('coin');
  });
});

describe('mergeInventoryRows', () => {
  it('returns empty when input empty', () => {
    expect(mergeInventoryRows([])).toEqual([]);
  });

  it('preserves order and first-seen name/icon', () => {
    const rows = [
      { id: 'longsword', name: 'Longsword', count: 1, icon: 'sword' },
      { id: 'shield', name: 'Shield', count: 1, icon: 'shield' },
    ];
    expect(mergeInventoryRows(rows)).toEqual(rows);
  });

  it('sums counts for duplicate ids', () => {
    const rows = [
      { id: 'javelin', name: 'Javelin', count: 1, icon: 'sword' },
      { id: 'javelin', name: 'Javelin', count: 3, icon: 'sword' },
    ];
    expect(mergeInventoryRows(rows)).toEqual([
      { id: 'javelin', name: 'Javelin', count: 4, icon: 'sword' },
    ]);
  });
});

describe('filterCompendiumByWildcard', () => {
  const COMP = {
    ...MINI_COMPENDIUM,
    equipment: {
      ...MINI_COMPENDIUM.equipment,
      weapons: [
        ...MINI_COMPENDIUM.equipment.weapons,
        {
          id: 'handaxe',
          name_en: 'Handaxe',
          name_ru: '',
          category: 'simple_melee',
          cost: { gp: 5 },
          damage: { dice: '1d6', type: 'slashing' },
          weight_lb: 2,
          properties: [],
          range_ft: {},
          source_url: '',
          srd_section: '',
        },
      ],
    },
  } as unknown as Compendium;

  it('"martial melee weapon" -> only martial_melee weapons', () => {
    const r = filterCompendiumByWildcard('martial melee weapon', COMP);
    expect(r.map((w) => w.id)).toEqual(['longsword']);
  });

  it('"simple weapon" -> all simple_* weapons', () => {
    const r = filterCompendiumByWildcard('simple weapon', COMP);
    expect(r.map((w) => w.id).sort()).toEqual(['crossbow-light', 'handaxe']);
  });

  it('unknown wildcard returns all weapons (defensive default)', () => {
    const r = filterCompendiumByWildcard('exotic weapon', COMP);
    expect(r.length).toBe(3);
  });
});

describe('resolveEquipmentSlots', () => {
  it('returns empty when no slots and no background items', () => {
    expect(resolveEquipmentSlots([], [], MINI_COMPENDIUM)).toEqual([]);
  });

  it('resolves a concrete slot (chain mail) into one inventory row', () => {
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'a',
        customName: 'chain mail',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ];
    const out = resolveEquipmentSlots(slots, [], MINI_COMPENDIUM);
    expect(out).toEqual([{ id: 'chain-mail', name: 'Chain Mail', count: 1, icon: 'shield' }]);
  });

  it('resolves a multi-item slot (light crossbow + 20 bolts) into two rows', () => {
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-1',
        category: 'gear',
        itemId: 'a',
        customName: 'light crossbow, 20 bolts',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ];
    const out = resolveEquipmentSlots(slots, [], MINI_COMPENDIUM);
    expect(out).toEqual([
      { id: 'crossbow-light', name: 'Crossbow, light', count: 1, icon: 'bow' },
      { id: 'crossbow-bolts-20', name: 'Crossbow bolts (20)', count: 1, icon: 'scroll' },
    ]);
  });

  it('uses resolvedItemIds when a wildcard slot has them set', () => {
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-2',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: ['longsword'],
      },
    ];
    const out = resolveEquipmentSlots(slots, [], MINI_COMPENDIUM);
    expect(out).toEqual([{ id: 'longsword', name: 'Longsword', count: 1, icon: 'sword' }]);
  });

  it('falls back to literal name when wildcard slot is unresolved', () => {
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-2',
        category: 'gear',
        itemId: 'b',
        customName: 'any martial melee weapon',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ];
    const out = resolveEquipmentSlots(slots, [], MINI_COMPENDIUM);
    expect(out).toHaveLength(1);
    expect(out[0]?.id).toBe('unresolved-class-2');
    expect(out[0]?.name).toBe('any martial melee weapon');
    expect(out[0]?.count).toBe(1);
  });

  it('merges background items into inventory and dedups across sources', () => {
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-0',
        category: 'gear',
        itemId: 'a',
        customName: 'longsword',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ];
    const bgItems = ['longsword'];
    const out = resolveEquipmentSlots(slots, bgItems, MINI_COMPENDIUM);
    expect(out).toEqual([{ id: 'longsword', name: 'Longsword', count: 2, icon: 'sword' }]);
  });

  it('handles "two handaxes" producing handaxe with count=2', () => {
    const comp = {
      ...MINI_COMPENDIUM,
      equipment: {
        ...MINI_COMPENDIUM.equipment,
        weapons: [
          ...MINI_COMPENDIUM.equipment.weapons,
          {
            id: 'handaxe',
            name_en: 'Handaxe',
            name_ru: '',
            category: 'simple_melee',
            cost: { gp: 5 },
            damage: { dice: '1d6', type: 'slashing' },
            weight_lb: 2,
            properties: [],
            range_ft: {},
            source_url: '',
            srd_section: '',
          },
        ],
      },
    } as unknown as Compendium;
    const slots: EquipmentSlot[] = [
      {
        slotId: 'class-3',
        category: 'gear',
        itemId: 'b',
        customName: 'two handaxes',
        fromBackground: false,
        resolvedItemIds: [],
      },
    ];
    const out = resolveEquipmentSlots(slots, [], comp);
    expect(out).toEqual([{ id: 'handaxe', name: 'Handaxe', count: 2, icon: 'sword' }]);
  });
});

const ACOLYTE_BG = {
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
} as unknown as Background;

describe('readBackgroundStartingEquipment', () => {
  it('returns the string list from a background', () => {
    expect(readBackgroundStartingEquipment(ACOLYTE_BG)).toEqual(['a holy symbol', 'vestments']);
  });
  it('returns [] for null', () => {
    expect(readBackgroundStartingEquipment(null)).toEqual([]);
  });
});

describe('computeGoldRows', () => {
  it('Package mode + Acolyte (15 gp) -> one gold row of 15', () => {
    expect(computeGoldRows({ equipmentMode: 'package', goldRemaining: 0 }, ACOLYTE_BG)).toEqual([
      { id: 'gold', name: 'Gold pieces', count: 15, icon: 'coin' },
    ]);
  });

  it('Gold mode + 42.5 residual -> one gold row of 42 (floored)', () => {
    expect(computeGoldRows({ equipmentMode: 'gold', goldRemaining: 42.5 }, ACOLYTE_BG)).toEqual([
      { id: 'gold', name: 'Gold pieces', count: 42, icon: 'coin' },
    ]);
  });

  it('Gold mode + 0 residual -> no row', () => {
    expect(computeGoldRows({ equipmentMode: 'gold', goldRemaining: 0 }, ACOLYTE_BG)).toEqual([]);
  });

  it('Package mode + bg without starting_gold -> no row', () => {
    const noGoldBg = { ...ACOLYTE_BG, starting_gold: undefined } as unknown as Background;
    expect(computeGoldRows({ equipmentMode: 'package', goldRemaining: 0 }, noGoldBg)).toEqual([]);
  });

  it('mode null -> no row', () => {
    expect(computeGoldRows({ equipmentMode: null, goldRemaining: 0 }, ACOLYTE_BG)).toEqual([]);
  });
});

describe('promoteIcon', () => {
  it('rewrites generic category icon (gear) to canonical icon for catalog item', () => {
    const before = { id: 'longsword', name: 'Longsword', count: 1, icon: 'weapon' };
    expect(promoteIcon(before, MINI_COMPENDIUM)).toEqual({
      id: 'longsword',
      name: 'Longsword',
      count: 1,
      icon: 'sword',
    });
  });

  it('rewrites armor category icon to shield', () => {
    const before = { id: 'chain-mail', name: 'Chain Mail', count: 1, icon: 'armor' };
    expect(promoteIcon(before, MINI_COMPENDIUM).icon).toBe('shield');
  });

  it('preserves row when catalog miss', () => {
    const before = { id: 'mystery', name: 'Mystery', count: 1, icon: 'scroll' };
    expect(promoteIcon(before, MINI_COMPENDIUM)).toEqual(before);
  });
});
