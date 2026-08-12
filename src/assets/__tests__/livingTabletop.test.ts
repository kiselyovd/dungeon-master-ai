import { statSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { HERO_ART, KEY_ART, NPC_ART, SAVE_ART, SCENE_ART, TOKEN_ART } from '../livingTabletop';

const WAVE_1: ReadonlyArray<readonly [string, number]> = [
  ['splash.webp', 900_000],
  ['onboarding.webp', 900_000],
  ['vtt-empty.webp', 800_000],
  ['hero-fighter.webp', 600_000],
  ['hero-wizard.webp', 600_000],
  ['hero-rogue.webp', 600_000],
  ['hero-cleric.webp', 600_000],
  ['token-fighter.png', 1_000_000],
  ['token-wizard.png', 1_000_000],
  ['token-rogue.png', 1_000_000],
  ['token-cleric.png', 1_000_000],
];

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

it.each(WAVE_1)('%s exists within its encoded budget', (name, maxBytes) => {
  const file = resolve(__dirname, `../living-tabletop/${name}`);
  const bytes = statSync(file).size;
  expect(bytes).toBeGreaterThan(10_000);
  expect(bytes).toBeLessThanOrEqual(maxBytes);
});
