import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type {
  AdventuringGear,
  Armor,
  Background,
  Class,
  Compendium,
  Cost,
  Weapon,
} from '../../api/srd';
import type { EquipmentMode, EquipmentSlot } from '../../state/charCreation';
import type { InventoryItem } from '../../state/pc';
import { useStore } from '../../state/useStore';
import { Icons } from '../../ui/Icons';
import { filterCompendiumByWildcard, parseEquipmentString } from './equipmentResolver';

interface RawStartingEquipmentEntry {
  option_letter: string;
  items: string[];
}

interface ChoiceGroup {
  /** stable id, e.g. `class-0` */
  slotId: string;
  /** options keyed by option_letter (e.g. 'a', 'b', 'c', or 'fixed') */
  options: Record<string, string[]>;
  /** preserve original option order for stable iteration */
  optionOrder: string[];
  /** true if this group is a single `fixed` entry (no real choice) */
  isFixed: boolean;
}

function readClassStartingEquipment(klass: Class): RawStartingEquipmentEntry[] {
  const raw = (klass as unknown as { starting_equipment?: unknown }).starting_equipment;
  if (!Array.isArray(raw)) return [];
  const out: RawStartingEquipmentEntry[] = [];
  for (const entry of raw) {
    if (typeof entry !== 'object' || entry === null) continue;
    const obj = entry as { option_letter?: unknown; items?: unknown };
    const letter = typeof obj.option_letter === 'string' ? obj.option_letter : null;
    if (!letter) continue;
    if (!Array.isArray(obj.items)) continue;
    const items: string[] = obj.items.filter((x): x is string => typeof x === 'string');
    out.push({ option_letter: letter, items });
  }
  return out;
}

function readBackgroundStartingEquipment(bg: Background): string[] {
  const raw = (bg as unknown as { starting_equipment?: unknown }).starting_equipment;
  if (!Array.isArray(raw)) return [];
  return raw.filter((x): x is string => typeof x === 'string');
}

/**
 * Group consecutive non-fixed entries (e.g. `a`/`b`/`c`) into one choice group.
 * Each `fixed` entry becomes its own group.
 */
export function buildChoiceGroups(entries: RawStartingEquipmentEntry[]): ChoiceGroup[] {
  const groups: ChoiceGroup[] = [];
  let current: ChoiceGroup | null = null;
  let idx = 0;

  function flush() {
    if (current) {
      groups.push(current);
      current = null;
    }
  }

  for (const entry of entries) {
    if (entry.option_letter === 'fixed') {
      flush();
      groups.push({
        slotId: `class-${idx++}`,
        options: { fixed: entry.items },
        optionOrder: ['fixed'],
        isFixed: true,
      });
      continue;
    }
    if (!current) {
      current = {
        slotId: `class-${idx++}`,
        options: {},
        optionOrder: [],
        isFixed: false,
      };
    }
    // If we have already seen this letter in the current group, the YAML restarted
    // a new choice group with letter 'a' again - flush and start fresh.
    if (current.options[entry.option_letter] !== undefined) {
      flush();
      current = {
        slotId: `class-${idx++}`,
        options: {},
        optionOrder: [],
        isFixed: false,
      };
    }
    current.options[entry.option_letter] = entry.items;
    current.optionOrder.push(entry.option_letter);
  }
  flush();
  return groups;
}

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

function buildSlotForGroup(group: ChoiceGroup, optionLetter: string): EquipmentSlot {
  const items = group.options[optionLetter] ?? [];
  return {
    slotId: group.slotId,
    category: 'gear',
    itemId: optionLetter,
    customName: items.join(', '),
    fromBackground: false,
    resolvedItemIds: [],
  };
}

export function EquipmentTab({ compendium }: { compendium: Compendium }) {
  const { t, i18n } = useTranslation('wizard');
  const mode = useStore((s) => s.charCreation.equipmentMode);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const inventory = useStore((s) => s.charCreation.equipmentInventory);
  const gold = useStore((s) => s.charCreation.goldRemaining);
  const strength = useStore((s) => s.charCreation.abilities.str);
  const classId = useStore((s) => s.charCreation.classId);
  const backgroundId = useStore((s) => s.charCreation.backgroundId);
  const slots = useStore((s) => s.charCreation.equipmentSlots);

  const klass = classId ? (compendium.classes.find((c) => c.id === classId) ?? null) : null;
  const bg = backgroundId
    ? (compendium.backgrounds.find((b) => b.id === backgroundId) ?? null)
    : null;
  const entries = klass ? readClassStartingEquipment(klass) : [];
  const groups = buildChoiceGroups(entries);
  const bgItems = bg ? readBackgroundStartingEquipment(bg) : [];

  // Hydrate equipmentSlots from class choice groups on entry to package mode,
  // or when the class changes after entering package mode. We recompute groups
  // inside the effect so its dependency list stays small (mode + classId), and
  // we don't accidentally re-hydrate on every render.
  useEffect(() => {
    if (mode !== 'package') return;
    if (!classId) return;
    const klassNow = compendium.classes.find((c) => c.id === classId) ?? null;
    if (!klassNow) return;
    const groupsNow = buildChoiceGroups(readClassStartingEquipment(klassNow));
    const slotsNow = useStore.getState().charCreation.equipmentSlots;
    if (
      slotsNow.length === groupsNow.length &&
      slotsNow.every((s, i) => s.slotId === groupsNow[i]?.slotId)
    ) {
      return;
    }
    const hydrated = groupsNow.map((g) => {
      const firstLetter = g.optionOrder[0] ?? 'fixed';
      return buildSlotForGroup(g, firstLetter);
    });
    setDraftField('equipmentSlots', hydrated);
  }, [mode, classId, compendium, setDraftField]);

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

  function pickOption(group: ChoiceGroup, optionLetter: string) {
    const next = slots.map((s) =>
      s.slotId === group.slotId ? buildSlotForGroup(group, optionLetter) : s,
    );
    setDraftField('equipmentSlots', next);
  }

  function pickWildcardItem(slotId: string, itemId: string) {
    const next = slots.map((s) => (s.slotId === slotId ? { ...s, resolvedItemIds: [itemId] } : s));
    setDraftField('equipmentSlots', next);
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
        <PackageMode
          classId={classId}
          groups={groups}
          slots={slots}
          backgroundItems={bgItems}
          backgroundPresent={bg !== null}
          compendium={compendium}
          onPick={pickOption}
          onPickWildcard={pickWildcardItem}
        />
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

interface PackageModeProps {
  classId: string | null;
  groups: ChoiceGroup[];
  slots: EquipmentSlot[];
  backgroundItems: string[];
  backgroundPresent: boolean;
  compendium: Compendium;
  onPick: (group: ChoiceGroup, optionLetter: string) => void;
  onPickWildcard: (slotId: string, itemId: string) => void;
}

function PackageMode({
  classId,
  groups,
  slots,
  backgroundItems,
  backgroundPresent,
  compendium,
  onPick,
  onPickWildcard,
}: PackageModeProps) {
  const { t } = useTranslation('wizard');

  if (!classId) {
    return (
      <div style={{ color: 'var(--color-text-muted)' }}>
        <p>{t('equipment_select_class_first')}</p>
      </div>
    );
  }

  const slotById = new Map(slots.map((s) => [s.slotId, s]));
  const hasEmptySlot = groups.some((g) => {
    const s = slotById.get(g.slotId);
    return !s || s.itemId === null;
  });

  return (
    <div>
      <section style={{ marginBottom: 16 }}>
        <h3>{t('equipment_class_items_title')}</h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {groups.map((g, i) => {
            const slot = slotById.get(g.slotId);
            const selected = slot?.itemId ?? g.optionOrder[0] ?? 'fixed';
            const resolved = slot?.customName ?? '';
            const selectId = `eq-${g.slotId}`;
            return (
              <div
                key={g.slotId}
                style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}
              >
                <label
                  htmlFor={selectId}
                  style={{ minWidth: 110, color: 'var(--color-text-muted)' }}
                >
                  {t('equipment_choice')} {i + 1}
                </label>
                <select
                  id={selectId}
                  value={selected}
                  disabled={g.isFixed}
                  onChange={(e) => onPick(g, e.target.value)}
                  style={{ padding: 6, minWidth: 200 }}
                >
                  {g.optionOrder.map((letter) => (
                    <option key={letter} value={letter}>
                      {(g.options[letter] ?? []).join(', ')}
                    </option>
                  ))}
                </select>
                {resolved.length > 0 && (
                  <span style={{ fontSize: 11, color: 'var(--color-text-muted)' }}>{resolved}</span>
                )}
                <WildcardChooser
                  slot={slot}
                  compendium={compendium}
                  onPick={onPickWildcard}
                  t={t}
                />
              </div>
            );
          })}
        </div>
      </section>

      {backgroundPresent && (
        <section style={{ marginBottom: 16 }}>
          <h3>{t('equipment_background_items_title')}</h3>
          <ul style={{ paddingLeft: 20, color: 'var(--color-text-muted)' }}>
            {backgroundItems.map((it) => (
              <li key={it}>{it}</li>
            ))}
          </ul>
        </section>
      )}

      {hasEmptySlot && (
        <div
          role="status"
          style={{
            fontSize: 12,
            color: 'var(--color-text-muted)',
            padding: '6px 10px',
            border: '1px solid rgba(212,175,55,0.3)',
            borderRadius: 4,
            display: 'inline-block',
          }}
        >
          {t('equipment_empty_warning')}
        </div>
      )}
    </div>
  );
}

interface WildcardChooserProps {
  slot: EquipmentSlot | undefined;
  compendium: Compendium;
  onPick: (slotId: string, itemId: string) => void;
  t: ReturnType<typeof useTranslation<'wizard'>>['t'];
}

function WildcardChooser({ slot, compendium, onPick, t }: WildcardChooserProps) {
  if (!slot) return null;
  const chunks = (slot.customName ?? '')
    .split(',')
    .map((c) => c.trim())
    .filter(Boolean);
  const wildcardChunks = chunks
    .map((c) => ({ raw: c, parsed: parseEquipmentString(c) }))
    .filter((x) => x.parsed.isWildcard);
  if (wildcardChunks.length === 0) return null;
  return (
    <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginLeft: 110 }}>
      {wildcardChunks.map((w, wi) => {
        const options = filterCompendiumByWildcard(w.parsed.nameKey, compendium);
        const selectedId = slot.resolvedItemIds[wi] ?? '';
        const ariaLabel = t('equipment_wildcard_aria', { wildcard: w.raw });
        return (
          <select
            key={`${slot.slotId}-wildcard-${wi}`}
            aria-label={ariaLabel}
            value={selectedId}
            onChange={(e) => onPick(slot.slotId, e.target.value)}
            style={{ padding: 6, minWidth: 200 }}
          >
            <option value="" disabled>
              {t('equipment_wildcard_placeholder')}
            </option>
            {options.map((opt) => (
              <option key={opt.id} value={opt.id}>
                {opt.name_en}
              </option>
            ))}
          </select>
        );
      })}
    </div>
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
                    <Icons.X size={10} />
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
