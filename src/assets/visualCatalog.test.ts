import { describe, expect, it } from 'vitest';
import { resolveHeroPortrait } from '../lib/heroPortraits';
import { HERO_ART } from './livingTabletop';
import { miniatureForPortrait, portraitAssetUrl, visualAsset } from './visualCatalog';
import { VISUAL_ASSET_ENTRIES } from './visualCatalog.generated';

describe('stable visual identities', () => {
  it('keeps assigned and legacy faces stable while resolving a matching whole miniature', () => {
    const id = 'hero.fighter.01.v2';
    expect(resolveHeroPortrait({ heroClass: 'fighter' })).toBe(HERO_ART.fighter);
    expect(resolveHeroPortrait({ heroClass: 'wizard', portraitAssetId: id })).toBe(
      portraitAssetUrl(id),
    );
    expect(miniatureForPortrait(id)).toMatchObject({
      id: 'miniature.fighter.01.v2',
      category: 'miniature',
      alpha: true,
    });
    expect(
      resolveHeroPortrait({ heroClass: 'fighter', portraitAssetId: 'portrait-missing-future-id' }),
    ).toBeNull();
    expect(visualAsset('map.bell_archive.v2', 'portrait')).toBeNull();
  });
  it('has unique immutable identities, hashes and valid portrait pairs', () => {
    const ids = VISUAL_ASSET_ENTRIES.map((asset) => asset.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const asset of VISUAL_ASSET_ENTRIES) {
      expect(asset.sha256).toMatch(/^[0-9a-f]{64}$/);
      expect(asset.width).toBeGreaterThan(0);
      expect(asset.bytes).toBeGreaterThan(0);
      if (asset.pairedAssetId) expect(visualAsset(asset.pairedAssetId, 'miniature')).not.toBeNull();
    }
  });
});
