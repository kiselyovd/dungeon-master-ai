import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { visualAsset } from '../assets/visualCatalog';
import { Button } from '../ui/Button';
import { Icons } from '../ui/Icons';
import { Modal } from '../ui/Modal';
import styles from './NpcMemoryGrid.module.css';

/** Cosmetic public image only; authoritative discovery/assignment is supplied by callers. */
export function NpcPortraitButton({
  assetId,
  name,
  archetype,
  unavailable = false,
}: {
  assetId: string | null;
  name: string;
  archetype?: string;
  unavailable?: boolean;
}) {
  const { t } = useTranslation('npc');
  const [expanded, setExpanded] = useState(false);
  const portrait = visualAsset(assetId, 'npc_portrait');
  return (
    <>
      <button
        type="button"
        className={styles.cardBanner}
        aria-label={t('open_portrait', { name })}
        disabled={!portrait}
        onClick={() => setExpanded(true)}
      >
        {portrait ? (
          <img
            src={portrait.uri}
            data-asset-id={assetId}
            data-archetype={archetype}
            alt=""
            className={styles.cardPortrait}
          />
        ) : (
          <span
            className={styles.unavailable}
            data-portrait-state={unavailable || assetId ? 'unavailable' : 'loading'}
          >
            <Icons.User size={32} />
            {unavailable || assetId ? t('portrait_unavailable') : ''}
          </span>
        )}
      </button>
      <Modal
        open={expanded && Boolean(portrait)}
        onClose={() => setExpanded(false)}
        title={name}
        footer={<Button onClick={() => setExpanded(false)}>{t('close_portrait')}</Button>}
      >
        {portrait && (
          <img
            src={portrait.uri}
            alt={t('portrait_alt', { name })}
            data-asset-id={assetId}
            className={styles.fullPortrait}
          />
        )}
      </Modal>
    </>
  );
}
