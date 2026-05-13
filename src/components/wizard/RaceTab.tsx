import { useTranslation } from 'react-i18next';
import type { Compendium } from '../../api/srd';
import { useStore } from '../../state/useStore';

export interface RaceTabProps {
  compendium: Compendium;
}

export function RaceTab({ compendium }: RaceTabProps) {
  const { t, i18n } = useTranslation('wizard');
  const raceId = useStore((s) => s.charCreation.raceId);
  const subraceId = useStore((s) => s.charCreation.subraceId);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const lang = i18n.language === 'ru' ? 'ru' : 'en';
  const selected = raceId ? (compendium.races.find((r) => r.id === raceId) ?? null) : null;

  return (
    <section>
      <h2>{t('race_picker_title')}</h2>
      <div className="dm-wizard-card-grid" role="radiogroup" aria-label={t('race_picker_title')}>
        {compendium.races.map((r) => (
          // biome-ignore lint/a11y/useSemanticElements: rich card content needs button surface
          <button
            key={r.id}
            type="button"
            role="radio"
            aria-checked={raceId === r.id}
            className={`dm-wizard-card${raceId === r.id ? ' is-selected' : ''}`}
            onClick={() => {
              setDraftField('raceId', r.id);
              setDraftField('subraceId', null);
            }}
          >
            <div style={{ fontFamily: 'Cinzel, serif', color: '#fff', fontSize: 16 }}>
              {lang === 'ru' ? r.name_ru : r.name_en}
            </div>
            <div style={{ fontSize: 11, color: 'var(--color-text-muted)', marginTop: 4 }}>
              {r.size} - {r.speed} ft
            </div>
          </button>
        ))}
      </div>

      {selected && selected.subraces.length > 0 && (
        <section style={{ marginTop: 24 }}>
          <h3>{t('subrace_picker_title')}</h3>
          <div
            style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}
            role="radiogroup"
            aria-label={t('subrace_picker_title')}
          >
            {selected.subraces.map((sr) => (
              // biome-ignore lint/a11y/useSemanticElements: pill control
              <button
                key={sr.id}
                type="button"
                role="radio"
                aria-checked={subraceId === sr.id}
                className={`dm-wizard-card${subraceId === sr.id ? ' is-selected' : ''}`}
                style={{ minWidth: 140 }}
                onClick={() => setDraftField('subraceId', sr.id)}
              >
                {lang === 'ru' ? sr.name_ru : sr.name_en}
              </button>
            ))}
          </div>
        </section>
      )}
    </section>
  );
}
