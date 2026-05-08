import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { JournalEntry } from '../state/journal';
import styles from './JournalViewer.module.css';

interface Props {
  entries: JournalEntry[];
  onClose: () => void;
}

/**
 * Full-screen journal overlay.
 * M3: functional plain viewer. Styled parchment renderer ships in M5.
 */
export function JournalViewer({ entries, onClose }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    containerRef.current?.focus();
  }, []);
  const { t } = useTranslation('journal');

  return (
    <div
      className={styles.overlay}
      role="dialog"
      aria-modal="true"
      aria-label={t('title')}
      onKeyDown={(e) => {
        if (e.key === 'Escape') {
          onClose();
        }
      }}
    >
      <div className={styles.container} tabIndex={-1} ref={containerRef}>
        <div className={styles.header}>
          <h2 className={styles.title}>{t('title')}</h2>
          <button
            type="button"
            className={styles.closeBtn}
            onClick={onClose}
            aria-label={t('close')}
          >
            &#x2715;
          </button>
        </div>
        <div className={styles.scroll}>
          {entries.length === 0 ? (
            <p className={styles.empty}>{t('no_entries')}</p>
          ) : (
            entries.map((entry) => (
              <article key={entry.id} className={styles.entry}>
                {entry.chapter && <h3 className={styles.chapter}>{entry.chapter}</h3>}
                <div
                  className={styles.prose}
                  // biome-ignore lint/security/noDangerouslySetInnerHtml: trusted backend HTML, sanitised upstream
                  dangerouslySetInnerHTML={{ __html: entry.entry_html }}
                />
                <time className={styles.timestamp} dateTime={entry.created_at}>
                  {new Date(entry.created_at).toLocaleString()}
                </time>
              </article>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
