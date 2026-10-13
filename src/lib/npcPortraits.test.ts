import { describe, expect, it } from 'vitest';
import { genericNpcPortraitId, npcArchetype } from './npcPortraits';

describe('public generic NPC appearance', () => {
  it.each([
    ['Innkeeper', 'innkeeper'],
    ['Трактирщик', 'innkeeper'],
    ['Стражница', 'guard'],
    ['Guard', 'guard'],
    ['Купец', 'merchant'],
    ['Merchant', 'merchant'],
    ['Плут', 'rogue'],
    ['Thief', 'rogue'],
    ['Волшебница', 'mage'],
    ['Mage', 'mage'],
    ['Жрица', 'priestess'],
    ['Priestess', 'priestess'],
    ['Рыцарь', 'knight'],
    ['Knight', 'knight'],
    ['Крестьянин', 'peasant'],
    ['Farmer', 'peasant'],
  ])('classifies %s as %s without using a display name', (role, archetype) => {
    expect(npcArchetype({ id: 'record', role })).toBe(archetype);
    expect(genericNpcPortraitId({ id: 'record', role })).toBe(`npc.${archetype}.v2`);
  });
  it('keeps an absent-role initial choice stable for the same record', () => {
    const first = genericNpcPortraitId({ id: 'same-record', role: '' });
    expect(genericNpcPortraitId({ id: 'same-record', role: '' })).toBe(first);
    expect(first).toMatch(
      /^npc\.(innkeeper|guard|merchant|rogue|mage|priestess|knight|peasant)\.v2$/,
    );
  });
});
