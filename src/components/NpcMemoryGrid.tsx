import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useClosingAnimation } from '../hooks/useClosingAnimation';
import { genericNpcPortraitId, npcArchetype } from '../lib/npcPortraits';
import { DISPOSITIONS, type Disposition, type NpcRecord } from '../state/npc';
import { npcAppearance } from '../state/npcAppearance';
import { ContextSurface } from './ContextSurface';
import styles from './NpcMemoryGrid.module.css';
import { NpcPortraitButton } from './NpcPortraitButton';

interface Props {
  embedded?: boolean;
  npcs: NpcRecord[];
  onClose: () => void;
}

const DISPOSITION_COLORS: Record<Disposition, string> = {
  friendly: 'var(--color-success)',
  neutral: 'var(--color-fg-secondary)',
  hostile: 'var(--color-danger)',
  unknown: 'var(--color-fg-muted)',
};

export function NpcMemoryGrid({ npcs, onClose, embedded = false }: Props) {
  const { t } = useTranslation('npc');
  const containerRef = useRef<HTMLDivElement>(null);
  const [search, setSearch] = useState('');
  const [filterDisposition, setFilterDisposition] = useState<Disposition | null>(null);
  const { isClosing, triggerClose } = useClosingAnimation(onClose);

  useEffect(() => {
    if (!embedded) containerRef.current?.focus();
  }, [embedded]);

  const filtered = npcs.filter((n) => {
    const matchSearch =
      n.name.toLowerCase().includes(search.toLowerCase()) ||
      n.role.toLowerCase().includes(search.toLowerCase());
    const matchDisp = filterDisposition === null || n.disposition === filterDisposition;
    return matchSearch && matchDisp;
  });

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
          <div className={styles.headerActions}>
            <input
              type="search"
              className={styles.search}
              placeholder={t('search_placeholder')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Escape' && search) {
                  e.stopPropagation();
                  setSearch('');
                }
              }}
              aria-label={t('search_placeholder')}
            />
            <button
              type="button"
              className={styles.closeBtn}
              onClick={triggerClose}
              aria-label={t('close')}
            >
              &#x2715;
            </button>
          </div>
        </div>
        <div className={styles.filters}>
          {DISPOSITIONS.map((d) => (
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
    </ContextSurface>
  );
}

function NpcCard({ npc }: { npc: NpcRecord }) {
  const { t } = useTranslation('npc');
  const archetype = npcArchetype(npc);
  const suggestedId = genericNpcPortraitId(npc);
  const key = `${npc.campaign_id}:${npc.id}`;
  const [assigned, setAssigned] = useState<{ key: string; id: string } | null>(null);
  const [failedKey, setFailedKey] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void npcAppearance(key, suggestedId)
      .then((id) => {
        if (alive) setAssigned({ key, id });
      })
      .catch(() => {
        if (alive) setFailedKey(key);
      });
    return () => {
      alive = false;
    };
  }, [key, suggestedId]);
  const assetId = assigned?.key === key ? assigned.id : null;
  return (
    <article className={styles.card}>
      <NpcPortraitButton
        assetId={assetId}
        name={npc.name}
        archetype={archetype}
        unavailable={failedKey === key}
      />
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
