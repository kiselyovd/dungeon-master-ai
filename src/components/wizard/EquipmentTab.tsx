import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AdventuringGear, Armor, Compendium, Cost, Weapon } from '../../api/srd';
import type { EquipmentMode } from '../../state/charCreation';
import type { InventoryItem } from '../../state/pc';
import { useStore } from '../../state/useStore';

type FilterChip = 'all' | 'weapons' | 'armor' | 'gear';

interface CatalogEntry {
  id: string;
  name_en: string;
  name_ru: string;
  costGp: number;
  weightLb: number;
  category: 'weapon' | 'armor' | 'gear';
}

function gpOf(cost: Cost | undefined): number {
  if (!cost) return 0;
  return (cost.gp ?? 0) + (cost.sp ?? 0) / 10 + (cost.cp ?? 0) / 100;
}

function buildCatalog(comp: Compendium): CatalogEntry[] {
  const out: CatalogEntry[] = [];
  for (const w of comp.equipment.weapons as Weapon[]) {
    out.push({
      id: w.id,
      name_en: w.name_en,
      name_ru: w.name_ru,
      costGp: gpOf(w.cost),
      weightLb: w.weight_lb,
      category: 'weapon',
    });
  }
  for (const a of comp.equipment.armor as Armor[]) {
    out.push({
      id: a.id,
      name_en: a.name_en,
      name_ru: a.name_ru,
      costGp: gpOf(a.cost),
      weightLb: a.weight_lb,
      category: 'armor',
    });
  }
  for (const g of comp.equipment.adventuring_gear as AdventuringGear[]) {
    out.push({
      id: g.id,
      name_en: g.name_en,
      name_ru: g.name_ru,
      costGp: gpOf(g.cost),
      weightLb: g.weight_lb,
      category: 'gear',
    });
  }
  return out;
}

function readStartingGold(comp: Compendium, classId: string | null): number {
  if (!classId) return 80;
  const klass = comp.classes.find((c) => c.id === classId);
  if (!klass) return 80;
  const raw = (klass as unknown as { starting_gold?: unknown }).starting_gold;
  return typeof raw === 'number' ? raw : 80;
}

export function EquipmentTab({ compendium }: { compendium: Compendium }) {
  const { t, i18n } = useTranslation('wizard');
  const mode = useStore((s) => s.charCreation.equipmentMode);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const inventory = useStore((s) => s.charCreation.equipmentInventory);
  const gold = useStore((s) => s.charCreation.goldRemaining);
  const strength = useStore((s) => s.charCreation.abilities.str);
  const classId = useStore((s) => s.charCreation.classId);

  function pickMode(m: EquipmentMode) {
    setDraftField('equipmentMode', m);
    if (m === 'gold') {
      const seedGold = readStartingGold(compendium, classId);
      setDraftField('goldRemaining', seedGold);
      setDraftField('equipmentInventory', []);
    } else {
      setDraftField('equipmentSlots', []);
    }
  }

  return (
    <section>
      <h2>{t('equipment_title')}</h2>
      <div
        role="radiogroup"
        aria-label={t('equipment_mode_label')}
        style={{ display: 'flex', gap: 8, marginBottom: 16 }}
      >
        {(['package', 'gold'] as EquipmentMode[]).map((m) => (
          // biome-ignore lint/a11y/useSemanticElements: card-style radio
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={mode === m}
            className={`dm-wizard-card${mode === m ? ' is-selected' : ''}`}
            onClick={() => pickMode(m)}
          >
            {t(`equipment_mode_${m}`)}
          </button>
        ))}
      </div>

      {mode === 'package' && (
        <div style={{ color: 'var(--color-text-muted)' }}>
          <p>{t('equipment_package_note')}</p>
        </div>
      )}

      {mode === 'gold' && (
        <GoldMode
          compendium={compendium}
          gold={gold}
          weightCap={strength * 15}
          inventory={inventory}
          lang={i18n.language === 'ru' ? 'ru' : 'en'}
          onAdd={(entry) => {
            setDraftField('equipmentInventory', [
              ...inventory,
              { id: entry.id, name: entry.name_en, count: 1, icon: entry.category },
            ]);
            setDraftField('goldRemaining', Number((gold - entry.costGp).toFixed(2)));
          }}
          onRemove={(item, costGp) => {
            setDraftField(
              'equipmentInventory',
              inventory.filter((it) => it.id !== item.id),
            );
            setDraftField('goldRemaining', Number((gold + costGp).toFixed(2)));
          }}
        />
      )}
    </section>
  );
}

interface GoldModeProps {
  compendium: Compendium;
  gold: number;
  weightCap: number;
  inventory: InventoryItem[];
  lang: 'en' | 'ru';
  onAdd: (entry: CatalogEntry) => void;
  onRemove: (item: InventoryItem, costGp: number) => void;
}

function GoldMode({
  compendium,
  gold,
  weightCap,
  inventory,
  lang,
  onAdd,
  onRemove,
}: GoldModeProps) {
  const { t } = useTranslation('wizard');
  const [chip, setChip] = useState<FilterChip>('all');
  const [search, setSearch] = useState('');
  const catalog = buildCatalog(compendium);
  const totalWeight = inventory.reduce((acc, it) => {
    const found = catalog.find((c) => c.id === it.id);
    return acc + (found?.weightLb ?? 0) * it.count;
  }, 0);

  const filtered = catalog.filter((e) => {
    if (chip !== 'all') {
      if (chip === 'weapons' && e.category !== 'weapon') return false;
      if (chip === 'armor' && e.category !== 'armor') return false;
      if (chip === 'gear' && e.category !== 'gear') return false;
    }
    if (search.length > 0) {
      const name = lang === 'ru' ? e.name_ru : e.name_en;
      if (!name.toLowerCase().includes(search.toLowerCase())) return false;
    }
    return true;
  });

  return (
    <div>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
          gap: 12,
          marginBottom: 12,
        }}
      >
        <div className="dm-wizard-card" style={{ cursor: 'default' }}>
          <div className="dm-wizard-live-stat-label">{t('gold_remaining')}</div>
          <div className="dm-wizard-live-stat-value">
            {gold.toFixed(1)} {t('gp')}
          </div>
        </div>
        <div className="dm-wizard-card" style={{ cursor: 'default' }}>
          <div className="dm-wizard-live-stat-label">{t('weight_carried')}</div>
          <div className="dm-wizard-live-stat-value">
            {totalWeight.toFixed(1)} / {weightCap} {t('lb')}
          </div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        {(['all', 'weapons', 'armor', 'gear'] as FilterChip[]).map((c) => (
          <button
            key={c}
            type="button"
            className={`dm-wizard-card${chip === c ? ' is-selected' : ''}`}
            style={{ padding: '4px 12px', fontSize: 12 }}
            onClick={() => setChip(c)}
          >
            {t(`filter_${c}`)}
          </button>
        ))}
      </div>

      <input
        type="search"
        placeholder={t('search_placeholder')}
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        style={{ width: '100%', padding: 8, marginBottom: 12 }}
      />

      <div className="dm-wizard-card-grid">
        {filtered.slice(0, 60).map((entry) => {
          const name = lang === 'ru' ? entry.name_ru : entry.name_en;
          const canAfford = gold >= entry.costGp;
          return (
            <div key={entry.id} className="dm-wizard-card" style={{ cursor: 'default' }}>
              <div style={{ fontFamily: 'Cinzel, serif', color: '#fff' }}>{name}</div>
              <div style={{ fontSize: 11, color: 'var(--color-text-muted)' }}>
                {entry.costGp.toFixed(2)} {t('gp')} - {entry.weightLb} {t('lb')}
              </div>
              <button
                type="button"
                className="dm-wizard-sparkle"
                disabled={!canAfford}
                onClick={() => onAdd(entry)}
                style={{ marginTop: 8 }}
              >
                {t('add_to_inventory')}
              </button>
            </div>
          );
        })}
      </div>

      {inventory.length > 0 && (
        <section style={{ marginTop: 24 }}>
          <h3>{t('current_inventory')}</h3>
          <ul style={{ listStyle: 'none', padding: 0 }}>
            {inventory.map((it) => {
              const found = catalog.find((c) => c.id === it.id);
              const cost = found?.costGp ?? 0;
              return (
                <li
                  key={it.id}
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    padding: 4,
                    borderBottom: '1px solid rgba(212,175,55,0.1)',
                  }}
                >
                  <span>{found ? (lang === 'ru' ? found.name_ru : found.name_en) : it.name}</span>
                  <button
                    type="button"
                    aria-label={t('remove_from_inventory')}
                    onClick={() => onRemove(it, cost)}
                  >
                    x
                  </button>
                </li>
              );
            })}
          </ul>
        </section>
      )}
    </div>
  );
}
