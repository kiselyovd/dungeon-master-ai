import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './ImageLightboxModal.module.css';

interface Props {
  src: string;
  alt: string;
  onClose: () => void;
}

/**
 * Full-screen image preview. Closes on Escape, on backdrop click, or on the
 * explicit close button. Click on the image itself does NOT dismiss
 * (`stopPropagation`) so the user can right-click + save without the modal
 * dismissing under them.
 */
export function ImageLightboxModal({ src, alt, onClose }: Props) {
  const { t } = useTranslation('chat');

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [onClose]);

  return (
    <div
      className={styles.backdrop}
      data-testid="lightbox-backdrop"
      onClick={onClose}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') onClose();
      }}
      role="dialog"
      aria-modal="true"
      aria-label={alt || 'image preview'}
      tabIndex={-1}
    >
      <div
        className={styles.frame}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => e.stopPropagation()}
        role="presentation"
      >
        <img src={src} alt={alt} className={styles.image} data-testid="lightbox-image" />
        <button
          type="button"
          aria-label={t('attachment_lightbox_close')}
          className={styles.close}
          onClick={onClose}
        >
          ×
        </button>
      </div>
    </div>
  );
}
