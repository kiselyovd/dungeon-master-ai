import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import {
  abilityMod,
  createPcSlice,
  type HeroClassId,
  type PcSlice,
  savingThrowMod,
  skillMod,
} from '../pc';

function freshStore() {
  return create<PcSlice>()((...a) => ({
    ...createPcSlice(...a),
  }));
}

describe('PcSlice', () => {
  it('starts with heroClass null and an empty character profile', () => {
    const store = freshStore();
    const pc = store.getState().pc;
    expect(pc.heroClass).toBeNull();
    expect(pc.name).toBeNull();
    expect(pc.race).toBeNull();
    expect(pc.inventory).toEqual([]);
  });

  it('setHeroClass with a known class id seeds the full preset', () => {
    const store = freshStore();
    store.getState().pc.setHeroClass('wizard');
    const pc = store.getState().pc;
    expect(pc.heroClass).toBe('wizard');
    expect(pc.name).toBe('Hero');
    expect(pc.race).toBe('High Elf');
    expect(pc.abilities.int).toBe(16);
    expect(pc.savingThrowProfs.int).toBe(true);
    expect(pc.skillProfs.arcana).toBe(true);
    expect(pc.inventory.length).toBeGreaterThan(0);
  });

  it('setHeroClass(null) resets the slice back to the empty PC profile', () => {
    const store = freshStore();
    store.getState().pc.setHeroClass('rogue');
    expect(store.getState().pc.heroClass).toBe('rogue');
    store.getState().pc.setHeroClass(null);
    const pc = store.getState().pc;
    expect(pc.heroClass).toBeNull();
    expect(pc.name).toBeNull();
    expect(pc.inventory).toEqual([]);
  });

  it('applyPreset seeds defaults for all four canonical classes', () => {
    const expectations: Array<{ klass: HeroClassId; name: string; race: string; ac: number }> = [
      { klass: 'fighter', name: 'Hero', race: 'Human', ac: 16 },
      { klass: 'wizard', name: 'Hero', race: 'High Elf', ac: 12 },
      { klass: 'rogue', name: 'Hero', race: 'Halfling', ac: 14 },
      { klass: 'cleric', name: 'Hero', race: 'Hill Dwarf', ac: 16 },
    ];
    for (const e of expectations) {
      const store = freshStore();
      store.getState().pc.applyPreset(e.klass);
      const pc = store.getState().pc;
      expect(pc.heroClass).toBe(e.klass);
      expect(pc.name).toBe(e.name);
      expect(pc.race).toBe(e.race);
      expect(pc.ac).toBe(e.ac);
      expect(pc.level).toBe(1);
      expect(pc.proficiencyBonus).toBe(2);
      expect(pc.hp).toBe(pc.hpMax);
    }
  });

  it('Fighter preset sets STR/CON saves and athletics + intimidation skills', () => {
    const store = freshStore();
    store.getState().pc.applyPreset('fighter');
    const pc = store.getState().pc;
    expect(pc.savingThrowProfs.str).toBe(true);
    expect(pc.savingThrowProfs.con).toBe(true);
    expect(pc.savingThrowProfs.dex).toBeUndefined();
    expect(pc.skillProfs.athletics).toBe(true);
    expect(pc.skillProfs.intimidation).toBe(true);
    expect(pc.inventory.find((i) => i.icon === 'sword')?.name).toBe('Longsword');
  });

  it('setHp clamps the new value into [0, hpMax]', () => {
    const store = freshStore();
    store.getState().pc.applyPreset('fighter');
    const max = store.getState().pc.hpMax;

    store.getState().pc.setHp(-5);
    expect(store.getState().pc.hp).toBe(0);

    store.getState().pc.setHp(max + 50);
    expect(store.getState().pc.hp).toBe(max);

    store.getState().pc.setHp(3);
    expect(store.getState().pc.hp).toBe(3);
  });

  it('setAc updates the AC field', () => {
    const store = freshStore();
    store.getState().pc.applyPreset('wizard');
    store.getState().pc.setAc(15);
    expect(store.getState().pc.ac).toBe(15);
  });

  it('addInventoryItem and removeInventoryItem mutate the inventory', () => {
    const store = freshStore();
    store.getState().pc.applyPreset('rogue');
    const before = store.getState().pc.inventory.length;
    store
      .getState()
      .pc.addInventoryItem({ id: 'rope', name: 'Hempen rope', count: 1, icon: 'scroll' });
    expect(store.getState().pc.inventory.length).toBe(before + 1);
    store.getState().pc.removeInventoryItem('rope');
    expect(store.getState().pc.inventory.length).toBe(before);
    expect(store.getState().pc.inventory.find((i) => i.id === 'rope')).toBeUndefined();
  });
});

describe('abilityMod', () => {
  it('returns the D&D 5e modifier for canonical scores', () => {
    expect(abilityMod(10)).toBe(0);
    expect(abilityMod(11)).toBe(0);
    expect(abilityMod(12)).toBe(1);
    expect(abilityMod(8)).toBe(-1);
    expect(abilityMod(7)).toBe(-2);
    expect(abilityMod(18)).toBe(4);
    expect(abilityMod(20)).toBe(5);
    expect(abilityMod(3)).toBe(-4);
  });
});

describe('skillMod / savingThrowMod', () => {
  it('adds proficiency bonus only when prof is true', () => {
    // STR 16 -> mod +3. With prof bonus +2 and prof flag => +5.
    expect(skillMod(16, false, 2)).toBe(3);
    expect(skillMod(16, true, 2)).toBe(5);
    expect(savingThrowMod(16, false, 2)).toBe(3);
    expect(savingThrowMod(16, true, 2)).toBe(5);

    // CHA 8 -> mod -1. With prof bonus +2 and prof flag => +1.
    expect(skillMod(8, true, 2)).toBe(1);
    expect(skillMod(8, false, 2)).toBe(-1);
  });
});
