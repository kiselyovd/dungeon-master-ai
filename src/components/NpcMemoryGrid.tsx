import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { NpcRecord } from '../state/npc';
import styles from './NpcMemoryGrid.module.css';

interface Props {
  npcs: NpcRecord[];
  onClose: () => void;
}

const DISPOSITION_COLORS: Record<string, string> = {
  friendly: 'var(--color-success)',
  neutral: 'var(--color-fg-secondary)',
  hostile: 'var(--color-danger)',
  unknown: 'var(--color-fg-muted)',
};

export function NpcMemoryGrid({ npcs, onClose }: Props) {
  const { t } = useTranslation('npc');
  const containerRef = useRef<HTMLDivElement>(null);
  const [search, setSearch] = useState('');
  const [filterDisposition, setFilterDisposition] = useState<string | null>(null);

  useEffect(() => {
    containerRef.current?.focus();
  }, []);

  const filtered = npcs.filter((n) => {
    const matchSearch =
      n.name.toLowerCase().includes(search.toLowerCase()) ||
      n.role.toLowerCase().includes(search.toLowerCase());
    const matchDisp = filterDisposition === null || n.disposition === filterDisposition;
    return matchSearch && matchDisp;
  });

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
          <div className={styles.headerActions}>
            <input
              type="search"
              className={styles.search}
              placeholder={t('search_placeholder')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              aria-label={t('search_placeholder')}
            />
            <button
              type="button"
              className={styles.closeBtn}
              onClick={onClose}
              aria-label={t('close')}
            >
              &#x2715;
            </button>
          </div>
        </div>
        <div className={styles.filters}>
          {(['friendly', 'neutral', 'hostile', 'unknown'] as const).map((d) => (
            <button
              key={d}
              type="button"
              className={`${styles.chip} ${filterDisposition === d ? styles.chipActive : ''}`}
              onClick={() => setFilterDisposition(filterDisposition === d ? null : d)}
              style={{ '--chip-color': DISPOSITION_COLORS[d] } as React.CSSProperties}
            >
              {t(`disposition_${d}`)}
            </button>
          ))}
        </div>
        {filtered.length === 0 ? (
          <p className={styles.empty}>{npcs.length === 0 ? t('no_npcs') : t('no_results')}</p>
        ) : (
          <div className={styles.grid}>
            {filtered.map((npc) => (
              <NpcCard key={npc.id} npc={npc} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function NpcCard({ npc }: { npc: NpcRecord }) {
  const { t } = useTranslation('npc');
  return (
    <article className={styles.card}>
      <div className={styles.cardBanner}>
        <div className={styles.cardAvatar} aria-hidden="true">
          {npc.name.charAt(0).toUpperCase()}
        </div>
      </div>
      <div className={styles.cardBody}>
        <h3 className={styles.cardName}>{npc.name}</h3>
        {npc.role && <p className={styles.cardRole}>{npc.role}</p>}
        <div className={styles.cardChips}>
          <span
            className={styles.dispositionChip}
            style={{ color: DISPOSITION_COLORS[npc.disposition] ?? 'inherit' }}
          >
            {t(`disposition_${npc.disposition}`)}
          </span>
        </div>
        {npc.facts.length > 0 && (
          <ul className={styles.facts}>
            {npc.facts.slice(0, 4).map((fact, i) => (
              <li key={i} className={styles.fact}>
                {fact.text}
              </li>
            ))}
            {npc.facts.length > 4 && (
              <li className={styles.factMore}>
                {t('more_facts', { count: npc.facts.length - 4 })}
              </li>
            )}
          </ul>
        )}
      </div>
    </article>
  );
}
