import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { NPC_ART } from '../assets/livingTabletop';
import { visualAsset } from '../assets/visualCatalog';
import { useClosingAnimation } from '../hooks/useClosingAnimation';
import { DISPOSITIONS, type Disposition, type NpcRecord } from '../state/npc';
import { npcAppearance } from '../state/npcAppearance';
import { ContextSurface } from './ContextSurface';
import styles from './NpcMemoryGrid.module.css';

type NpcArchetype =
  | 'innkeeper'
  | 'guard'
  | 'merchant'
  | 'rogue'
  | 'mage'
  | 'priestess'
  | 'knight'
  | 'peasant';

const ARCHETYPE_PORTRAIT: Record<NpcArchetype, string> = NPC_ART;

const ARCHETYPE_ORDER: readonly NpcArchetype[] = [
  'innkeeper',
  'guard',
  'merchant',
  'rogue',
  'mage',
  'priestess',
  'knight',
  'peasant',
];

// Keyword -> archetype heuristic. Order matters: more specific roles win.
const ROLE_KEYWORD_MAP: ReadonlyArray<readonly [RegExp, NpcArchetype]> = [
  [/innkeep|tavern|barkeep|bartender|host/i, 'innkeeper'],
  [/priest|cleric|nun|abbot|bishop|temple|shrine/i, 'priestess'],
  [/wizard|mage|sorcer|warlock|witch|arcanist|scholar/i, 'mage'],
  [/knight|paladin|champion|captain|baron|lord/i, 'knight'],
  [/guard|soldier|sentinel|watchman|sergeant|warden/i, 'guard'],
  [/rogue|thief|bandit|brigand|cutpurse|assassin|spy/i, 'rogue'],
  [/merchant|trader|shopkeep|vendor|smith|fletcher|alchem|crafts/i, 'merchant'],
  [/farm|peasant|villager|commoner|laborer|miner|sailor/i, 'peasant'],
];

// FNV-1a 32-bit; deterministic, no external deps.
function hash(str: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < str.length; i += 1) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

function npcArchetype(npc: Pick<NpcRecord, 'id' | 'role'>): NpcArchetype {
  for (const [pattern, archetype] of ROLE_KEYWORD_MAP) {
    if (pattern.test(npc.role)) return archetype;
  }
  // Preserve the original eight-way assignment when the production catalog expands.
  const idx = hash(npc.id) % 8;
  return ARCHETYPE_ORDER[idx] ?? 'peasant';
}

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
  const key = `${npc.campaign_id}:${npc.id}`;
  const [assigned, setAssigned] = useState<{ key: string; id: string } | null>(null);
  useEffect(() => {
    let alive = true;
    void npcAppearance(key, `npc.${archetype}.legacy.v1`)
      .then((id) => {
        if (alive) setAssigned({ key, id });
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [key, archetype]);
  const assetId = assigned?.key === key ? assigned.id : `npc.${archetype}.legacy.v1`;
  return (
    <article className={styles.card}>
      <div className={styles.cardBanner}>
        <img
          src={visualAsset(assetId)?.uri ?? ARCHETYPE_PORTRAIT[archetype]}
          data-asset-id={assetId}
          alt=""
          className={styles.cardPortrait}
          data-archetype={archetype}
        />
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
