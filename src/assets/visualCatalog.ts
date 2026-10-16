import { VISUAL_ASSET_ENTRIES } from './visualCatalog.generated';

export type VisualAsset = (typeof VISUAL_ASSET_ENTRIES)[number];
export type VisualAssetCategory = VisualAsset['category'];
const assets: ReadonlyMap<string, VisualAsset> = new Map(
  VISUAL_ASSET_ENTRIES.map((asset) => [asset.id, asset]),
);

export function visualAsset(
  id: string | null | undefined,
  category?: VisualAssetCategory,
): VisualAsset | null {
  if (!id) return null;
  const asset = assets.get(id);
  return asset && (category === undefined || asset.category === category) ? asset : null;
}
export function portraitAssetUrl(id: string | null | undefined): string | null {
  return visualAsset(id, 'portrait')?.uri ?? null;
}
export function portraitFromBackendAsset(id: string): VisualAsset | null {
  return (
    VISUAL_ASSET_ENTRIES.find(
      (asset) => asset.category === 'portrait' && asset.backendAssetId === id,
    ) ?? null
  );
}
export function portraitChoices(heroClass: string | null): VisualAsset[] {
  return VISUAL_ASSET_ENTRIES.filter(
    (asset) => asset.category === 'portrait' && (!heroClass || asset.heroClass === heroClass),
  );
}
export function miniatureForPortrait(id: string | null | undefined): VisualAsset | null {
  const portrait = visualAsset(id, 'portrait');
  return portrait ? visualAsset(portrait.pairedAssetId, 'miniature') : null;
}
