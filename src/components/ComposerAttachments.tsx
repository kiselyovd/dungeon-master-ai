import { useTranslation } from 'react-i18next';
import type { StagedImage } from '../state/chat';
import styles from './ComposerAttachments.module.css';

interface Props {
  items: StagedImage[];
  onRemove: (index: number) => void;
}

/**
 * Horizontal strip of 64x64 thumbnails for images staged in the composer.
 * Hidden when the staged list is empty. Each thumbnail has an x button that
 * fires `onRemove(index)`.
 */
export function ComposerAttachments({ items, onRemove }: Props) {
  const { t } = useTranslation('chat');
  if (items.length === 0) return null;
  return (
    <ul className={styles.strip}>
      {items.map((item, i) => (
        <li key={`staged-${i}-${item.name ?? 'image'}`} className={styles.thumb}>
          <img src={item.dataUrl} alt={item.name ?? 'image'} className={styles.image} />
          <button
            type="button"
            aria-label={t('remove_image')}
            className={styles.remove}
            onClick={() => onRemove(i)}
          >
            ×
          </button>
        </li>
      ))}
    </ul>
  );
}
