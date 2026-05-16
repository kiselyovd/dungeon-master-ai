import { describe, expect, it } from 'vitest';
import type { Compendium } from '../../../api/srd';
import { iconFor, lookupItemByName, parseEquipmentString } from '../equipmentResolver';

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
