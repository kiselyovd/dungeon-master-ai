import { describe, expect, it } from 'vitest';
import { EMPTY_DRAFT } from '../../../state/charCreation';
import { computeLiveSheet } from '../computeLiveSheet';

const fixture = {
  races: [
    {
      id: 'human',
      name_en: 'Human',
      name_ru: 'Человек',
      speed: 30,
      ability_score_increases: { str: 1, dex: 1, con: 1, int: 1, wis: 1, cha: 1 },
      subraces: [],
    },
    {
      id: 'hill-dwarf',
      name_en: 'Hill Dwarf',
      name_ru: 'Холмовой Дварф',
      speed: 25,
      ability_score_increases: { wis: 1, con: 2 },
      subraces: [],
    },
  ],
  classes: [
    {
      id: 'fighter',
      name_en: 'Fighter',
      name_ru: 'Воин',
      hit_die: 10,
      saving_throw_proficiencies: ['str', 'con'],
    },
    {
      id: 'wizard',
      name_en: 'Wizard',
      name_ru: 'Маг',
      hit_die: 6,
      saving_throw_proficiencies: ['int', 'wis'],
    },
  ],
  backgrounds: [
    {
      id: 'acolyte',
      name_en: 'Acolyte',
      name_ru: 'Послушник',
      skill_proficiencies: ['insight', 'religion'],
    },
  ],
} as never;

describe('computeLiveSheet', () => {
  it('returns placeholder for empty draft', () => {
    const sheet = computeLiveSheet(EMPTY_DRAFT, fixture);
    expect(sheet.className).toBe(null);
    expect(sheet.hp).toBe(null);
    expect(sheet.placeholder).toBe('pick_class_to_begin');
  });

  it('computes HP from class hit die + CON modifier', () => {
    const draft = {
      ...EMPTY_DRAFT,
      classId: 'fighter',
      abilities: { str: 16, dex: 12, con: 14, int: 10, wis: 13, cha: 11 },
    };
    const sheet = computeLiveSheet(draft, fixture);
    expect(sheet.hp).toBe(12); // 10 + (14-10)/2 = 12
    expect(sheet.hpMax).toBe(12);
  });

  it('computes initiative from DEX modifier', () => {
    const draft = {
      ...EMPTY_DRAFT,
      classId: 'fighter',
      abilities: { ...EMPTY_DRAFT.abilities, dex: 14 },
    };
    expect(computeLiveSheet(draft, fixture).initiative).toBe(2);
  });

  it('picks speed from race', () => {
    const draft = { ...EMPTY_DRAFT, classId: 'fighter', raceId: 'hill-dwarf' };
    expect(computeLiveSheet(draft, fixture).speedFt).toBe(25);
  });

  it('marks proficient saving throws', () => {
    const draft = { ...EMPTY_DRAFT, classId: 'fighter' };
    const sheet = computeLiveSheet(draft, fixture);
    expect(sheet.savingThrows.str.proficient).toBe(true);
    expect(sheet.savingThrows.con.proficient).toBe(true);
    expect(sheet.savingThrows.dex.proficient).toBe(false);
  });

  it('applies background skill profs', () => {
    const draft = {
      ...EMPTY_DRAFT,
      classId: 'wizard',
      backgroundId: 'acolyte',
      skillProfs: ['arcana'],
    };
    const sheet = computeLiveSheet(draft, fixture);
    expect(sheet.skills.arcana?.proficient).toBe(true);
    expect(sheet.skills.insight?.proficient).toBe(true);
    expect(sheet.skills.athletics?.proficient).toBe(false);
  });

  it('returns empty inventory preview when no items', () => {
    const draft = { ...EMPTY_DRAFT, classId: 'fighter' };
    expect(computeLiveSheet(draft, fixture).inventoryPreview).toEqual([]);
  });

  it('truncates inventory preview to 3 + counter', () => {
    const inv = [1, 2, 3, 4, 5].map((i) => ({
      id: `i${i}`,
      name: `Item ${i}`,
      count: 1,
      icon: 'coin',
    }));
    const draft = { ...EMPTY_DRAFT, classId: 'fighter', equipmentInventory: inv };
    const sheet = computeLiveSheet(draft, fixture);
    expect(sheet.inventoryPreview).toHaveLength(3);
    expect(sheet.inventoryOverflow).toBe(2);
  });
});
