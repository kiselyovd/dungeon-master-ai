/**
 * heroPortraits.ts - shared portrait asset map for hero classes.
 *
 * Centralises the portrait PNG imports so both CharacterSheet and HeroStep
 * resolve portraits from one place. CharacterSheet previously owned these
 * imports; they live here now to avoid duplication.
 */

import portraitCleric from '../assets/char-portrait-cleric.png';
import portraitFighter from '../assets/char-portrait-fighter.png';
import portraitPaladin from '../assets/char-portrait-paladin.png';
import portraitRogue from '../assets/char-portrait-rogue.png';
import portraitWizard from '../assets/char-portrait-wizard.png';
import type { HeroClassId } from '../state/pc';

/**
 * Map from hero class id (or 'paladin' for legacy compat) to the imported
 * portrait PNG data URL. CharacterSheet uses the 'paladin' key; the four
 * canonical onboarding classes are fighter / wizard / rogue / cleric.
 */
export const HERO_PORTRAIT: Record<HeroClassId | 'paladin', string> = {
  fighter: portraitFighter,
  wizard: portraitWizard,
  rogue: portraitRogue,
  cleric: portraitCleric,
  paladin: portraitPaladin,
};
