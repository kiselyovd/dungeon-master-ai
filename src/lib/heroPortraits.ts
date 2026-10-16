/**
 * heroPortraits.ts - shared portrait asset map for hero classes.
 *
 * Centralises the portrait PNG imports so both CharacterSheet and HeroStep
 * resolve portraits from one place. CharacterSheet previously owned these
 * imports; they live here now to avoid duplication.
 */

import { HERO_ART } from '../assets/livingTabletop';
import type { HeroClassId } from '../state/pc';

/**
 * Map from hero class id (or 'paladin' for persisted legacy saves) to the
 * living-tabletop portrait URL. Paladins use the fighter art until they become
 * a selectable class with their own semantic asset.
 */
export const HERO_PORTRAIT: Record<HeroClassId | 'paladin', string> = {
  ...HERO_ART,
  paladin: HERO_ART.fighter,
};
