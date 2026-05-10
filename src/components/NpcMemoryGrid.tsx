import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import npcGuard from '../assets/npc-fallback-guard.png';
import npcInnkeeper from '../assets/npc-fallback-innkeeper.png';
import npcKnight from '../assets/npc-fallback-knight.png';
import npcMage from '../assets/npc-fallback-mage.png';
import npcMerchant from '../assets/npc-fallback-merchant.png';
import npcPeasant from '../assets/npc-fallback-peasant.png';
import npcPriestess from '../assets/npc-fallback-priestess.png';
import npcRogue from '../assets/npc-fallback-rogue.png';
import { DISPOSITIONS, type Disposition, type NpcRecord } from '../state/npc';
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

const ARCHETYPE_PORTRAIT: Record<NpcArchetype, string> = {
  innkeeper: npcInnkeeper,
  guard: npcGuard,
  merchant: npcMerchant,
  rogue: npcRogue,
  mage: npcMage,
  priestess: npcPriestess,
  knight: npcKnight,
  peasant: npcPeasant,
};

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
  const idx = hash(npc.id) % ARCHETYPE_ORDER.length;
  return ARCHETYPE_ORDER[idx] ?? 'peasant';
}

interface Props {
  npcs: NpcRecord[];
  onClose: () => void;
}

const DISPOSITION_COLORS: Record<Disposition, string> = {
  friendly: 'var(--color-success)',
  neutral: 'var(--color-fg-secondary)',
  hostile: 'var(--color-danger)',
  unknown: 'var(--color-fg-muted)',
};

export function NpcMemoryGrid({ npcs, onClose }: Props) {
  const { t } = useTranslation('npc');
  const containerRef = useRef<HTMLDivElement>(null);
  const [search, setSearch] = useState('');
  const [filterDisposition, setFilterDisposition] = useState<Disposition | null>(null);

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
              onClick={onClose}
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
    </div>
  );
}

function NpcCard({ npc }: { npc: NpcRecord }) {
  const { t } = useTranslation('npc');
  const archetype = npcArchetype(npc);
  return (
    <article className={styles.card}>
      <div className={styles.cardBanner}>
        <img
          src={ARCHETYPE_PORTRAIT[archetype]}
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
