import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { portraitAssetUrl } from '../../assets/visualCatalog';
import { Icons } from '../../ui/Icons';
import styles from './GameTable.module.css';
import type { TableActor } from './model/types';

export function PartyStrip({
  actors,
  renderPortrait,
}: {
  actors: readonly TableActor[];
  renderPortrait?: ((actor: TableActor) => ReactNode) | undefined;
}) {
  const { t } = useTranslation('rooms');
  return (
    <fieldset className={styles.party} aria-label={t('party')}>
      {actors
        .filter((actor) => actor.allegiance !== 'enemy')
        .map((actor) => (
          <div
            className={styles.actor}
            key={actor.id}
            data-actor-id={actor.id}
            data-allegiance={actor.allegiance}
          >
            <span className={styles.portrait}>
              {renderPortrait ? (
                renderPortrait(actor)
              ) : actor.portraitAssetId && !actor.portraitMediaRef ? (
                <img src={portraitAssetUrl(actor.portraitAssetId) ?? undefined} alt="" />
              ) : (
                <Icons.User size={20} />
              )}
            </span>
            <span>
              <strong>{actor.name}</strong>
              <small>
                {t('hp')} {actor.hp}/{actor.maxHp} · {t('ac')} {actor.ac}
              </small>
            </span>
          </div>
        ))}
    </fieldset>
  );
}
