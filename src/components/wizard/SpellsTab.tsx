import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Class, Compendium, Spell } from '../../api/srd';
import { useStore } from '../../state/useStore';

interface SpellcastingInfo {
  cantrips_known?: number;
  spells_known?: number;
  spells_prepared?: number;
}

function readSpellcasting(klass: Class): SpellcastingInfo | null {
  const raw = (klass as unknown as { spellcasting?: unknown }).spellcasting;
  if (typeof raw !== 'object' || raw === null) return null;
  const obj = raw as {
    cantrips_known?: unknown;
    spells_known?: unknown;
    spells_prepared?: unknown;
  };
  const info: SpellcastingInfo = {};
  if (typeof obj.cantrips_known === 'number') info.cantrips_known = obj.cantrips_known;
  if (typeof obj.spells_known === 'number') info.spells_known = obj.spells_known;
  if (typeof obj.spells_prepared === 'number') info.spells_prepared = obj.spells_prepared;
  return info;
}

export function SpellsTab({ compendium }: { compendium: Compendium }) {
  const { t, i18n } = useTranslation('wizard');
  const [filter, setFilter] = useState('');
  const classId = useStore((s) => s.charCreation.classId);
  const spells = useStore((s) => s.charCreation.spells);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const lang = i18n.language === 'ru' ? 'ru' : 'en';

  const klass = classId ? (compendium.classes.find((c) => c.id === classId) ?? null) : null;
  const spellcasting = klass ? readSpellcasting(klass) : null;

  if (!classId || !spellcasting) {
    return (
      <section>
        <h2>{t('spells_title')}</h2>
        <p style={{ color: 'var(--color-text-muted)' }}>{t('spells_not_caster')}</p>
      </section>
    );
  }

  const cantripQuota = spellcasting.cantrips_known ?? 0;
  const spellQuota = spellcasting.spells_known ?? spellcasting.spells_prepared ?? 0;
  const cantrips = compendium.spells.filter((s) => s.level === 0 && s.classes.includes(classId));
  const level1 = compendium.spells.filter((s) => s.level === 1 && s.classes.includes(classId));
  const filt = (list: Spell[]) =>
    list.filter((s) =>
      (lang === 'ru' ? s.name_ru : s.name_en).toLowerCase().includes(filter.toLowerCase()),
    );

  function toggleSpell(spell: Spell, kind: 'cantrips' | 'level1') {
    const current = spells[kind];
    const quota = kind === 'cantrips' ? cantripQuota : spellQuota;
    if (current.includes(spell.id)) {
      setDraftField('spells', { ...spells, [kind]: current.filter((id) => id !== spell.id) });
    } else if (current.length < quota) {
      setDraftField('spells', { ...spells, [kind]: [...current, spell.id] });
    }
  }

  return (
    <section>
      <h2>{t('spells_title')}</h2>
      <input
        type="search"
        placeholder={t('spells_filter_placeholder')}
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        style={{ width: '100%', padding: 8, marginBottom: 16 }}
      />
      <h3>{t('cantrips_section', { chosen: spells.cantrips.length, quota: cantripQuota })}</h3>
      <div className="dm-wizard-card-grid">
        {filt(cantrips).map((s) => (
          <button
            key={s.id}
            type="button"
            className={`dm-wizard-card${spells.cantrips.includes(s.id) ? ' is-selected' : ''}`}
            onClick={() => toggleSpell(s, 'cantrips')}
          >
            <div>{lang === 'ru' ? s.name_ru : s.name_en}</div>
            <div style={{ fontSize: 10, color: 'var(--color-text-muted)' }}>
              {s.school} - {s.casting_time}
            </div>
          </button>
        ))}
      </div>
      <h3 style={{ marginTop: 24 }}>
        {t('level1_section', { chosen: spells.level1.length, quota: spellQuota })}
      </h3>
      <div className="dm-wizard-card-grid">
        {filt(level1).map((s) => (
          <button
            key={s.id}
            type="button"
            className={`dm-wizard-card${spells.level1.includes(s.id) ? ' is-selected' : ''}`}
            onClick={() => toggleSpell(s, 'level1')}
          >
            <div>{lang === 'ru' ? s.name_ru : s.name_en}</div>
            <div style={{ fontSize: 10, color: 'var(--color-text-muted)' }}>
              {s.school} - {s.casting_time}
            </div>
          </button>
        ))}
      </div>
    </section>
  );
}
