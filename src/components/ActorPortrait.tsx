import { useTranslation } from 'react-i18next';
import { useMediaImage } from '../features/media/useMediaImage';
import { legacyPortraitUrl } from '../lib/heroPortraits';
import { Icons } from '../ui/Icons';
import { type ActorVisualPresentation, actorBuiltinAppearance } from './actorVisuals';

/** Reuses the owned-reference cache for party art, including other public actors.
 * Scope is supplied by the app adapter; the shared table never owns credentials. */
export function ActorPortrait({ actor }: { actor: ActorVisualPresentation }) {
  const { t } = useTranslation('wizard');
  const media = useMediaImage(actor.portraitMediaRef ?? null, actor.mediaScope ?? null, 'portrait');
  const builtin = actorBuiltinAppearance(actor);
  const url = actor.portraitMediaRef
    ? media.url
    : (builtin.portrait ?? (actor.portraitUrl ? legacyPortraitUrl(actor.portraitUrl) : null));
  return url ? (
    <img src={url} alt="" />
  ) : (
    <span title={builtin.assigned ? t('portrait_unavailable') : undefined}>
      <Icons.User size={20} />
    </span>
  );
}
