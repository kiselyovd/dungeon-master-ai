import { describe, expect, it } from 'vitest';
import { HERO_ART, KEY_ART, NPC_ART, SAVE_ART, SCENE_ART, TOKEN_ART } from '../livingTabletop';

describe('living tabletop asset registry', () => {
  it('exposes every reachable semantic role', () => {
    expect(Object.keys(HERO_ART)).toEqual(['fighter', 'wizard', 'rogue', 'cleric']);
    expect(Object.keys(TOKEN_ART)).toEqual(['fighter', 'wizard', 'rogue', 'cleric']);
    expect(Object.keys(NPC_ART)).toEqual([
      'innkeeper',
      'guard',
      'merchant',
      'rogue',
      'mage',
      'priestess',
      'knight',
      'peasant',
    ]);
    expect(Object.keys(SAVE_ART)).toEqual(['combat', 'exploration', 'dialog', 'npc']);
    expect(Object.keys(SCENE_ART)).toEqual(['combat', 'dialog', 'exploration', 'dungeon']);
    expect(Object.keys(KEY_ART)).toEqual(['splash', 'onboarding', 'vttEmpty']);
  });
});
