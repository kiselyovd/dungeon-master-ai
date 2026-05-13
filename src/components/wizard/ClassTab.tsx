import { useTranslation } from 'react-i18next';
import type { Class, Compendium } from '../../api/srd';
import { useCharacterAssist } from '../../hooks/useCharacterAssist';
import { useStore } from '../../state/useStore';

interface SubclassEntry {
  id: string;
  name_en: string;
  name_ru: string;
}

function readSubclasses(klass: Class): SubclassEntry[] {
  const raw = (klass as unknown as { subclasses?: unknown }).subclasses;
  if (!Array.isArray(raw)) return [];
  return raw.filter((s): s is SubclassEntry => {
    if (typeof s !== 'object' || s === null) return false;
    const o = s as { id?: unknown; name_en?: unknown; name_ru?: unknown };
    return (
      typeof o.id === 'string' && typeof o.name_en === 'string' && typeof o.name_ru === 'string'
    );
  });
}

function subclassAtLevel(klass: Class): number | null {
  const raw = (klass as unknown as { subclass_at_level?: unknown }).subclass_at_level;
  return typeof raw === 'number' ? raw : null;
}

export interface ClassTabProps {
  compendium: Compendium;
}

export function ClassTab({ compendium }: ClassTabProps) {
  const { t, i18n } = useTranslation('wizard');
  const classId = useStore((s) => s.charCreation.classId);
  const subclassId = useStore((s) => s.charCreation.subclassId);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const isAssisting = useStore((s) => s.charCreation.isAssisting);
  const { surpriseMe } = useCharacterAssist();
  const lang = i18n.language === 'ru' ? 'ru' : 'en';

  const selected = classId ? (compendium.classes.find((c) => c.id === classId) ?? null) : null;
  const subclasses = selected && subclassAtLevel(selected) === 1 ? readSubclasses(selected) : [];

  return (
    <section>
      <header
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <h2>{t('class_picker_title')}</h2>
        <button
          type="button"
          className="dm-wizard-btn-primary"
          disabled={isAssisting}
          onClick={() => {
            void surpriseMe();
          }}
        >
          {t('surprise_me')}
        </button>
      </header>
      <div className="dm-wizard-card-grid" role="radiogroup" aria-label={t('class_picker_title')}>
        {compendium.classes.map((c) => (
          // biome-ignore lint/a11y/useSemanticElements: rich card content (name + hit-die + ability) needs a button surface; native <input type="radio"> cannot host this layout
          <button
            key={c.id}
            type="button"
            role="radio"
            aria-checked={classId === c.id}
            className={`dm-wizard-card${classId === c.id ? ' is-selected' : ''}`}
            onClick={() => {
              setDraftField('classId', c.id);
              setDraftField('subclassId', null);
            }}
          >
            <div style={{ fontFamily: 'Cinzel, serif', color: '#fff', fontSize: 16 }}>
              {lang === 'ru' ? c.name_ru : c.name_en}
            </div>
            <div style={{ fontSize: 11, color: 'var(--color-text-muted)', marginTop: 4 }}>
              d{c.hit_die} HP - {c.primary_ability.join(', ')}
            </div>
          </button>
        ))}
      </div>

      {selected && subclasses.length > 0 && (
        <section style={{ marginTop: 24 }}>
          <h3>{t('subclass_picker_title')}</h3>
          <div
            className="dm-wizard-card-grid"
            role="radiogroup"
            aria-label={t('subclass_picker_title')}
          >
            {subclasses.map((sc) => (
              // biome-ignore lint/a11y/useSemanticElements: rich card content (subclass name) needs a button surface; native <input type="radio"> cannot host this layout
              <button
                key={sc.id}
                type="button"
                role="radio"
                aria-checked={subclassId === sc.id}
                className={`dm-wizard-card${subclassId === sc.id ? ' is-selected' : ''}`}
                onClick={() => setDraftField('subclassId', sc.id)}
              >
                <div style={{ fontFamily: 'Cinzel, serif', color: '#fff' }}>
                  {lang === 'ru' ? sc.name_ru : sc.name_en}
                </div>
              </button>
            ))}
          </div>
        </section>
      )}
    </section>
  );
}
