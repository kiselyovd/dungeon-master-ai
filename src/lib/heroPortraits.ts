/**
 * heroPortraits.ts - shared portrait asset map for hero classes.
 *
 * Centralises the portrait PNG imports so both CharacterSheet and HeroStep
 * resolve portraits from one place. CharacterSheet previously owned these
 * imports; they live here now to avoid duplication.
 */

import type { MediaReference } from '../api/contracts/mediaAsset';
import { HERO_ART } from '../assets/livingTabletop';
import { portraitAssetUrl } from '../assets/visualCatalog';
import { VISUAL_ASSET_ENTRIES } from '../assets/visualCatalog.generated';
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
export function legacyPortraitUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  if (
    url.length <= 8 * 1024 * 1024 &&
    /^data:image\/(?:png|jpeg|webp);base64,[A-Za-z0-9+/]+={0,2}$/.test(url)
  )
    return url;
  return VISUAL_ASSET_ENTRIES.some((asset) => asset.category === 'portrait' && asset.uri === url)
    ? url
    : null;
}

export function resolveHeroPortrait(character: {
  portraitAssetId?: string | null;
  portraitUrl?: string | null;
  heroClass?: string | null;
  portraitMediaRef?: MediaReference | null;
}): string | null {
  if (character.portraitMediaRef)
    return character.portraitMediaRef.source === 'bundled'
      ? portraitAssetUrl(character.portraitMediaRef.asset_id)
      : null;
  if (character.portraitAssetId) return portraitAssetUrl(character.portraitAssetId);
  if (character.portraitUrl) return legacyPortraitUrl(character.portraitUrl);
  return character.heroClass
    ? (HERO_PORTRAIT[character.heroClass as HeroClassId | 'paladin'] ?? null)
    : null;
}
