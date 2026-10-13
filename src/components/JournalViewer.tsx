import DOMPurify from 'dompurify';
import { useEffect, useMemo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useClosingAnimation } from '../hooks/useClosingAnimation';
import type { JournalEntry } from '../state/journal';
import { ContextSurface } from './ContextSurface';
import styles from './JournalViewer.module.css';

interface Props {
  embedded?: boolean;
  entries: JournalEntry[];
  onClose: () => void;
}

/**
 * Full-screen journal overlay.
 * M3: functional plain viewer. Styled parchment renderer ships in M5.
 */
export function JournalViewer({ entries, onClose, embedded = false }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const { isClosing, triggerClose } = useClosingAnimation(onClose);
  useEffect(() => {
    if (!embedded) containerRef.current?.focus();
  }, [embedded]);
  const { t, i18n } = useTranslation('journal');

  // Sanitise once per (id + html) pair so re-renders don't re-purify the
  // same entry. Backend sanitises upstream too; this is defense in depth.
  const sanitisedEntries = useMemo(
    () =>
      entries.map((entry) => ({
        entry,
        safeHtml: DOMPurify.sanitize(entry.entry_html, { USE_PROFILES: { html: true } }),
      })),
    [entries],
  );

  return (
    <ContextSurface
      embedded={embedded}
      label={t('title')}
      className={embedded ? styles.embedded : styles.overlay}
      state={isClosing ? 'closing' : 'open'}
      onEscape={triggerClose}
    >
      <div className={styles.container} tabIndex={-1} ref={containerRef}>
        <div className={styles.header}>
          <h2 className={styles.title}>{t('title')}</h2>
          <button
            type="button"
            className={styles.closeBtn}
            onClick={triggerClose}
            aria-label={t('close')}
          >
            &#x2715;
          </button>
        </div>
        <div className={styles.scroll}>
          {sanitisedEntries.length === 0 ? (
            <p className={styles.empty}>{t('no_entries')}</p>
          ) : (
            sanitisedEntries.map(({ entry, safeHtml }) => (
              <article key={entry.id} className={styles.entry}>
                {entry.chapter && <h3 className={styles.chapter}>{entry.chapter}</h3>}
                <div
                  className={styles.prose}
                  // biome-ignore lint/security/noDangerouslySetInnerHtml: HTML sanitised by DOMPurify (defense in depth on top of backend sanitisation)
                  dangerouslySetInnerHTML={{ __html: safeHtml }}
                />
                <time className={styles.timestamp} dateTime={entry.created_at}>
                  {new Date(entry.created_at).toLocaleString(i18n.language, {
                    year: 'numeric',
                    month: 'short',
                    day: 'numeric',
                    hour: '2-digit',
                    minute: '2-digit',
                  })}
                </time>
              </article>
            ))
          )}
        </div>
      </div>
    </ContextSurface>
  );
}
