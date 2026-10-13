import type { MediaReference, MediaScope } from '../api/contracts/mediaAsset';
import { miniatureForPortrait, portraitAssetUrl } from '../assets/visualCatalog';

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
