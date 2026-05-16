import { describe, expect, it } from 'vitest';
import { parseEquipmentString } from '../equipmentResolver';

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
