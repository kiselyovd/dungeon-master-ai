/**
 * heroPortraits.ts - shared portrait asset map for hero classes.
 *
 * Centralises the portrait PNG imports so both CharacterSheet and HeroStep
 * resolve portraits from one place. CharacterSheet previously owned these
 * imports; they live here now to avoid duplication.
 */

import portraitPaladin from '../assets/char-portrait-paladin.png';
import { HERO_ART } from '../assets/livingTabletop';
import type { HeroClassId } from '../state/pc';

/**
 * Map from hero class id (or 'paladin' for legacy compat) to the imported
 * portrait PNG data URL. CharacterSheet uses the 'paladin' key; the four
 * canonical onboarding classes are fighter / wizard / rogue / cleric.
 */
export const HERO_PORTRAIT: Record<HeroClassId | 'paladin', string> = {
  ...HERO_ART,
  paladin: portraitPaladin,
};
