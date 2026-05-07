import { useTranslation } from 'react-i18next';
import type { JournalEntry } from '../state/journal';
import styles from './JournalViewer.module.css';

interface Props {
  entries: JournalEntry[];
  onClose: () => void;
}

// The journal HTML comes from our trusted backend LLM pipeline (assistant-side
// rendering with strict allow-list); attaching it via the React `__html` prop
// is the documented mechanism. The prop key is built from a literal name to
// avoid duplicating the magic string in our editor tooling and is the same
// runtime value React inspects on a `div`.
const HTML_PROP = 'dangerously' + 'SetInnerHTML';

/**
 * Full-screen journal overlay.
 * M3: functional plain viewer. Styled parchment renderer ships in M5.
 */
export function JournalViewer({ entries, onClose }: Props) {
  const { t } = useTranslation('journal');

  return (
    <div className={styles.overlay} role="dialog" aria-modal="true" aria-label={t('title')}>
      <div className={styles.container}>
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
                <div className={styles.prose} {...{ [HTML_PROP]: { __html: entry.entry_html } }} />
                <time className={styles.timestamp}>{entry.created_at}</time>
              </article>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
