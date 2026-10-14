import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import type { AdventureProjection } from '../api/contracts/adventure';
import { visualAsset } from '../assets/visualCatalog';
import { usePresentationMotion } from '../hooks/usePresentationMotion';
import { Button } from '../ui/Button';
import { DecorativeImage } from '../ui/DecorativeImage';
import { Modal } from '../ui/Modal';
import styles from './SceneArtwork.module.css';

export interface SceneArtworkPresentation {
  assetId: string;
  location: AdventureProjection['state']['location'];
}
export function sceneArtworkAsset(presentation: SceneArtworkPresentation) {
  const { assetId, location } = presentation;
  // A stale binding must not display another, possibly unrevealed, location.
  if (!new RegExp(`^scene\\.bell\\.${location}\\.v[1-9][0-9]*$`).test(assetId)) return null;
  return visualAsset(assetId, 'scene_art');
}

/** One public current-scene image; it never supplies a battle map or actor. */
export function SceneArtwork({
  presentation,
  stage,
}: {
  presentation: SceneArtworkPresentation;
  stage: boolean;
}) {
  const { t } = useTranslation('adventure');
  return (
    <PaintingView
      asset={sceneArtworkAsset(presentation)}
      assetId={presentation.assetId}
      location={presentation.location}
      title={t(`locations.${presentation.location}`)}
      scope={`${presentation.location}:${presentation.assetId}`}
      stage={stage}
    />
  );
}
/** Uncalibrated media is a scene preview, never a tactical terrain layer. */
export function DecorativeSceneArtwork({ src, stage }: { src: string; stage: boolean }) {
  const { t } = useTranslation('adventure');
  return (
    <PaintingView
      asset={{ uri: src, id: 'decorative-scene' }}
      assetId="decorative-scene"
      location="decorative"
      title={t('scene_preview')}
      scope={src}
      stage={stage}
    />
  );
}
function PaintingView({
  asset,
  assetId,
  location,
  title,
  scope,
  stage,
}: {
  asset: { uri: string; id: string } | null;
  assetId: string;
  location: string;
  title: string;
  scope: string;
  stage: boolean;
}) {
  const { t } = useTranslation('adventure');
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [unavailableId, setUnavailableId] = useState<string | null>(null);
  const motion = usePresentationMotion();
  const observed = useRef(scope);
  const [arrival, setArrival] = useState<string | null>(null);
  useLayoutEffect(() => {
    const changed = observed.current !== scope;
    observed.current = scope;
    if (!motion.active || motion.reduced) {
      setArrival(null);
      return;
    }
    if (changed) setArrival(scope);
  }, [scope, motion.active, motion.reduced]);
  useEffect(() => setExpandedId((current) => (current === scope ? current : null)), [scope]);
  const unavailable = !asset || unavailableId === assetId;
  const image = (preview: boolean) =>
    asset ? (
      <DecorativeImage
        src={asset.uri}
        assetId={asset.id}
        key={scope}
        className={
          preview
            ? styles.full
            : `${styles.painting}${arrival === scope ? ` ${styles.arrival}` : ''}`
        }
        onAvailabilityChange={(available) => setUnavailableId(available ? null : assetId)}
      />
    ) : null;
  return (
    <>
      {stage && (
        <section
          className={styles.stage}
          data-testid="current-scene-art"
          data-location={location}
          aria-label={t('scene_current', { location: title })}
        >
          {image(false)}
          <div className={`${styles.caption}${arrival === scope ? ` ${styles.arrival}` : ''}`}>
            <h2>{title}</h2>
            {unavailable && <p role="status">{t('scene_unavailable')}</p>}
          </div>
        </section>
      )}
      <div className={styles.previewTrigger}>
        <Button
          onClick={() => setExpandedId(scope)}
          disabled={!asset}
          aria-label={t('scene_open', { location: title })}
        >
          {t('scene_preview')}
        </Button>
      </div>
      {createPortal(
        <Modal
          open={expandedId === scope && Boolean(asset)}
          title={title}
          onClose={() => setExpandedId(null)}
          footer={<Button onClick={() => setExpandedId(null)}>{t('scene_close')}</Button>}
        >
          <div data-testid="scene-art-preview" data-location={location}>
            {image(true)}
            {unavailable && <p role="status">{t('scene_unavailable')}</p>}
          </div>
        </Modal>,
        document.body,
      )}
    </>
  );
}
