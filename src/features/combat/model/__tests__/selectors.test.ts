import { expect, it } from 'vitest';
import { isPlayerToken } from '../selectors';

it('an NPC sharing the hero name is not the owned character', () => {
  const npc = { id: 'npc', name: 'Mira', hp: 8, maxHp: 8, ac: 12, x: 9, y: 8, conditions: [] };
  const hero = { ...npc, id: 'hero-token', characterId: 'hero-character' };
  expect(isPlayerToken(npc, 'hero-character', 'Mira')).toBe(false);
  expect(isPlayerToken(hero, 'hero-character', 'Mira')).toBe(true);
});
