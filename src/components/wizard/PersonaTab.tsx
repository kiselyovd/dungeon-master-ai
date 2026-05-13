import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AssistField } from '../../api/characterAssist';
import type { Background, Compendium, Race, RaceTrait, Subrace } from '../../api/srd';
import { useCharacterAssist } from '../../hooks/useCharacterAssist';
import type { PersonalityFlag, PersonalityFlagSlotId } from '../../state/charCreation';
import { useStore } from '../../state/useStore';

const ALIGNMENTS: readonly (readonly string[])[] = [
  ['LG', 'NG', 'CG'],
  ['LN', 'TN', 'CN'],
  ['LE', 'NE', 'CE'],
];

const ALIGNMENT_IDS: readonly string[] = ['LG', 'NG', 'CG', 'LN', 'TN', 'CN', 'LE', 'NE', 'CE'];

const BG_TRAIT_CAP = 6;
const RACE_TRAIT_CAP = 4;

interface BackgroundCharacteristics {
  personality_traits?: unknown;
  bonds?: unknown;
}

function parseBackgroundChars(bg: Background | undefined): {
  traits: string[];
  bonds: string[];
} {
  if (!bg) return { traits: [], bonds: [] };
  const raw = bg.suggested_characteristics as BackgroundCharacteristics | null | undefined;
  if (!raw || typeof raw !== 'object') return { traits: [], bonds: [] };
  const pt = Array.isArray(raw.personality_traits)
    ? (raw.personality_traits.filter((x): x is string => typeof x === 'string') as string[])
    : [];
  const bonds = Array.isArray(raw.bonds)
    ? (raw.bonds.filter((x): x is string => typeof x === 'string') as string[])
    : [];
  return { traits: pt.slice(0, BG_TRAIT_CAP), bonds: bonds.slice(0, BG_TRAIT_CAP) };
}

function parseRaceTraitNames(race: Race | undefined, lang: 'en' | 'ru'): string[] {
  if (!race) return [];
  return race.traits
    .slice(0, RACE_TRAIT_CAP)
    .map((t: RaceTrait) => (lang === 'ru' ? t.name_ru : t.name_en))
    .filter((s): s is string => typeof s === 'string' && s.length > 0);
}

function splitAlignmentTendency(text: string | undefined): string[] {
  if (!text) return [];
  // Split on sentence boundaries (".", "!", "?") followed by space or end-of-string.
  const parts = text
    .split(/(?<=[.!?])\s+/)
    .map((s) => s.trim())
    .filter((s) => s.length > 0);
  // Drop trailing punctuation for cleaner dropdown labels.
  return parts.map((s) => s.replace(/[.!?]+$/, '')).filter((s) => s.length > 0);
}

function parseRaceQuirks(
  race: Race | undefined,
  subrace: Subrace | undefined,
  lang: 'en' | 'ru',
): string[] {
  if (!race) return [];
  const fragments = splitAlignmentTendency(race.alignment_tendency);
  if (fragments.length >= 1) return fragments;
  if (subrace) {
    return subrace.additional_traits
      .map((t) => (lang === 'ru' ? t.name_ru : t.name_en))
      .filter((s): s is string => typeof s === 'string' && s.length > 0);
  }
  return [];
}

function getAlignmentPool(
  t: (key: string) => string,
  alignmentId: string | null,
  kind: 'traits' | 'quirks',
): string[] {
  if (!alignmentId || !ALIGNMENT_IDS.includes(alignmentId)) return [];
  const out: string[] = [];
  // We seed 4 entries per pool. Read 0..3.
  for (let i = 0; i < 6; i += 1) {
    const key = `align_pool.${alignmentId}.${kind}.${i}`;
    const value = t(key);
    if (value && value !== key) {
      out.push(value);
    } else {
      break;
    }
  }
  return out;
}

const SLOT_DEFS: readonly {
  slotId: PersonalityFlagSlotId;
  source: PersonalityFlag['source'];
  labelKey: string;
}[] = [
  { slotId: 'bg-trait', source: 'background', labelKey: 'flag_bg_trait' },
  { slotId: 'bg-bond', source: 'background', labelKey: 'flag_bg_bond' },
  { slotId: 'align-trait', source: 'alignment', labelKey: 'flag_align_trait' },
  { slotId: 'align-quirk', source: 'alignment', labelKey: 'flag_align_quirk' },
  { slotId: 'race-trait', source: 'race', labelKey: 'flag_race_trait' },
  { slotId: 'race-quirk', source: 'race', labelKey: 'flag_race_quirk' },
];

export function PersonaTab(props: { compendium: Compendium }) {
  const { compendium } = props;
  const { t, i18n } = useTranslation('wizard');
  const lang: 'en' | 'ru' = i18n.language === 'ru' ? 'ru' : 'en';

  const name = useStore((s) => s.charCreation.name);
  const ideals = useStore((s) => s.charCreation.ideals);
  const bonds = useStore((s) => s.charCreation.bonds);
  const flaws = useStore((s) => s.charCreation.flaws);
  const backstory = useStore((s) => s.charCreation.backstory);
  const alignment = useStore((s) => s.charCreation.alignment);
  const backgroundId = useStore((s) => s.charCreation.backgroundId);
  const raceId = useStore((s) => s.charCreation.raceId);
  const subraceId = useStore((s) => s.charCreation.subraceId);
  const personalityFlags = useStore((s) => s.charCreation.personalityFlags);
  const isAssisting = useStore((s) => s.charCreation.isAssisting);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const { generateField } = useCharacterAssist();

  // Local UI state: which slots are currently in "custom" entry mode.
  // A slot enters custom mode when the user picks the Custom... option; it stays
  // in custom mode until the user picks a preset or clears.
  const [customMode, setCustomMode] = useState<Record<PersonalityFlagSlotId, boolean>>({
    'bg-trait': false,
    'bg-bond': false,
    'align-trait': false,
    'align-quirk': false,
    'race-trait': false,
    'race-quirk': false,
  });

  const background = compendium?.backgrounds?.find((b) => b.id === backgroundId);
  const race = compendium?.races?.find((r) => r.id === raceId);
  const subrace = race?.subraces.find((sr) => sr.id === subraceId);

  const bgChars = parseBackgroundChars(background);
  const raceTraitNames = parseRaceTraitNames(race, lang);
  const raceQuirks = parseRaceQuirks(race, subrace, lang);
  const alignTraits = getAlignmentPool(t, alignment, 'traits');
  const alignQuirks = getAlignmentPool(t, alignment, 'quirks');

  function poolFor(slotId: PersonalityFlagSlotId): string[] {
    switch (slotId) {
      case 'bg-trait':
        return bgChars.traits;
      case 'bg-bond':
        return bgChars.bonds;
      case 'align-trait':
        return alignTraits;
      case 'align-quirk':
        return alignQuirks;
      case 'race-trait':
        return raceTraitNames;
      case 'race-quirk':
        return raceQuirks;
    }
  }

  function valueFor(slotId: PersonalityFlagSlotId): string {
    return personalityFlags.find((f) => f.slotId === slotId)?.flag ?? '';
  }

  function upsertFlag(
    slotId: PersonalityFlagSlotId,
    source: PersonalityFlag['source'],
    flag: string,
  ): void {
    const existing = personalityFlags.filter((f) => f.slotId !== slotId);
    if (flag.length === 0) {
      setDraftField('personalityFlags', existing);
      return;
    }
    setDraftField('personalityFlags', [...existing, { slotId, source, flag }]);
  }

  function clearFlag(slotId: PersonalityFlagSlotId): void {
    setDraftField(
      'personalityFlags',
      personalityFlags.filter((f) => f.slotId !== slotId),
    );
  }

  function sparkle(field: AssistField) {
    return (
      <button
        type="button"
        className="dm-wizard-sparkle"
        disabled={isAssisting}
        onClick={() => {
          void generateField(field);
        }}
      >
        {t('generate')}
      </button>
    );
  }

  return (
    <section>
      <h2>{t('persona_title')}</h2>

      <div style={{ marginBottom: 16 }}>
        <h3 className="dm-wizard-live-section-label">{t('personality_flags_title')}</h3>
        {SLOT_DEFS.map((def) => {
          const pool = poolFor(def.slotId);
          const current = valueFor(def.slotId);
          const isPreset = current.length > 0 && pool.includes(current);
          // A row is in custom mode when either the user explicitly picked Custom...
          // or the stored value isn't in the preset pool (and isn't empty).
          const inCustomMode = customMode[def.slotId] || (current.length > 0 && !isPreset);
          const selectId = `dm-persona-flag-${def.slotId}`;
          const customInputId = `dm-persona-flag-${def.slotId}-custom`;
          const selectValue = inCustomMode ? '__custom__' : isPreset ? current : '';
          return (
            <div
              key={def.slotId}
              className="dm-flag-row"
              style={{ display: 'flex', gap: 8, alignItems: 'center', marginTop: 8 }}
            >
              <label htmlFor={selectId} style={{ minWidth: 140 }}>
                {t(def.labelKey)}
              </label>
              <select
                id={selectId}
                value={selectValue}
                onChange={(e) => {
                  const v = e.target.value;
                  if (v === '') {
                    setCustomMode((m) => ({ ...m, [def.slotId]: false }));
                    clearFlag(def.slotId);
                  } else if (v === '__custom__') {
                    setCustomMode((m) => ({ ...m, [def.slotId]: true }));
                    // Preserve any prior free-text; otherwise leave the store untouched
                    // until the user types something into the inline input.
                  } else {
                    setCustomMode((m) => ({ ...m, [def.slotId]: false }));
                    upsertFlag(def.slotId, def.source, v);
                  }
                }}
                style={{ flex: 1, padding: 6 }}
              >
                <option value="">--</option>
                {pool.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
                <option value="__custom__">{t('custom_option')}</option>
              </select>
              {inCustomMode && (
                <input
                  id={customInputId}
                  aria-label={`${t(def.labelKey)} custom`}
                  value={isPreset ? '' : current}
                  onChange={(e) => upsertFlag(def.slotId, def.source, e.target.value)}
                  style={{ flex: 1, padding: 6 }}
                />
              )}
            </div>
          );
        })}
      </div>

      <div style={{ marginBottom: 16 }}>
        <label htmlFor="dm-persona-name" className="dm-wizard-live-section-label">
          {t('name')}
        </label>
        <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
          <input
            id="dm-persona-name"
            value={name}
            onChange={(e) => setDraftField('name', e.target.value)}
            style={{ flex: 1, padding: 8 }}
          />
          {sparkle('name')}
        </div>
      </div>

      <div style={{ marginBottom: 16 }}>
        <div className="dm-wizard-live-section-label">{t('alignment')}</div>
        <table style={{ borderCollapse: 'collapse', marginTop: 4 }}>
          <tbody>
            {ALIGNMENTS.map((row) => (
              <tr key={row.join('-')}>
                {row.map((a) => (
                  <td key={a} style={{ padding: 2 }}>
                    <button
                      type="button"
                      className={`dm-wizard-card${alignment === a ? ' is-selected' : ''}`}
                      style={{ minWidth: 56, textAlign: 'center' }}
                      onClick={() => setDraftField('alignment', a)}
                    >
                      {a}
                    </button>
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {(
        [
          ['ideals', ideals],
          ['bonds', bonds],
          ['flaws', flaws],
          ['backstory', backstory],
        ] as const
      ).map(([field, value]) => (
        <div key={field} style={{ marginBottom: 16 }}>
          <label htmlFor={`dm-persona-${field}`} className="dm-wizard-live-section-label">
            {t(field)}
          </label>
          <div style={{ display: 'flex', gap: 8, marginTop: 4 }}>
            <textarea
              id={`dm-persona-${field}`}
              value={value}
              onChange={(e) => setDraftField(field, e.target.value)}
              style={{ flex: 1, minHeight: 60, padding: 8 }}
            />
            {sparkle(field as AssistField)}
          </div>
        </div>
      ))}
    </section>
  );
}
