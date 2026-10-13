import type { MediaReference, MediaScope } from '../api/contracts/mediaAsset';
import { miniatureForPortrait, portraitAssetUrl, visualAsset } from '../assets/visualCatalog';

const OPPONENT_ASSETS: Readonly<Record<string, string>> = {
  water_echo: 'opponent.water_echo.v2',
  goblin: 'opponent.goblin.v2',
  skeleton: 'opponent.skeleton.v2',
  zombie: 'opponent.zombie.v2',
  wolf: 'opponent.wolf.v2',
  bandit: 'opponent.bandit.v2',
  orc: 'opponent.orc.v2',
  giant_rat: 'opponent.giant_rat.v2',
};

/** Only explicit public metadata chooses an opponent. Unknown/unavailable art
 * stays a neutral marker, including future IDs from another app version. */
export function opponentAppearance(appearance: string | undefined) {
  const id = appearance ? OPPONENT_ASSETS[appearance] : null;
  const asset = visualAsset(id, 'miniature');
  return asset && 'spriteBounds' in asset ? asset : null;
}

/** Align the measured silhouette's ground point with the existing token base.
 * Transparent margins remain intact; this changes only the pointer-free art. */
export function opponentFrame(
  asset: NonNullable<ReturnType<typeof opponentAppearance>>,
  cellSize: number,
) {
  const bounds = asset.spriteBounds;
  const scale = Math.min(
    (cellSize * 1.65) / (bounds.right - bounds.left + 1),
    (cellSize * 2.15) / (bounds.bottom - bounds.top + 1),
  );
  return {
    width: asset.width * scale,
    height: asset.height * scale,
    bottom: cellSize * 0.08 - (asset.height - bounds.bottom - 1) * scale,
    transform: `translateX(-${(100 * (bounds.left + bounds.right + 1)) / (2 * asset.width)}%)`,
  };
}

export interface ActorVisualPresentation {
  characterId: string | null;
  heroClass: string | null;
  portraitAssetId?: string | null;
  portraitMediaRef?: MediaReference | null;
  /** Legacy appearance is scoped to the owning actor during migration only. */
  portraitUrl?: string | null;
  mediaScope?: MediaScope | null;
}
export type ActorVisualMap = Readonly<Record<string, ActorVisualPresentation>>;

/** An assigned unknown/custom portrait cannot silently become another face or
 * an invented matching miniature. Legacy class defaults apply only without art. */
export function actorBuiltinAppearance(actor: ActorVisualPresentation) {
  const assigned = Boolean(actor.portraitAssetId || actor.portraitMediaRef || actor.portraitUrl);
  const id =
    actor.portraitAssetId ??
    (!assigned && actor.heroClass ? `hero.${actor.heroClass}.legacy.v1` : null);
  const portrait = id ? portraitAssetUrl(id) : null;
  const miniature =
    id && !actor.portraitMediaRef && !actor.portraitUrl ? miniatureForPortrait(id) : null;
  return {
    portrait,
    miniature: miniature?.uri ?? null,
    miniatureId: miniature?.id ?? null,
    assigned,
  };
}
