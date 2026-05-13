import { useTranslation } from 'react-i18next';
import type { Compendium } from '../../api/srd';
import { useStore } from '../../state/useStore';

export interface BackgroundTabProps {
  compendium: Compendium;
}

export function BackgroundTab({ compendium }: BackgroundTabProps) {
  const { t, i18n } = useTranslation('wizard');
  const backgroundId = useStore((s) => s.charCreation.backgroundId);
  const setDraftField = useStore((s) => s.charCreation.setDraftField);
  const lang = i18n.language === 'ru' ? 'ru' : 'en';
  return (
    <section>
      <h2>{t('background_picker_title')}</h2>
      <div
        className="dm-wizard-card-grid"
        role="radiogroup"
        aria-label={t('background_picker_title')}
      >
        {compendium.backgrounds.map((b) => (
          // biome-ignore lint/a11y/useSemanticElements: rich card content
          <button
            key={b.id}
            type="button"
            role="radio"
            aria-checked={backgroundId === b.id}
            className={`dm-wizard-card${backgroundId === b.id ? ' is-selected' : ''}`}
            onClick={() => setDraftField('backgroundId', b.id)}
          >
            <div style={{ fontFamily: 'Cinzel, serif', color: '#fff' }}>
              {lang === 'ru' ? b.name_ru : b.name_en}
            </div>
            <div style={{ fontSize: 11, color: 'var(--color-text-muted)', marginTop: 6 }}>
              {b.skill_proficiencies.join(' / ')}
            </div>
          </button>
        ))}
      </div>
      <p style={{ marginTop: 16, color: 'var(--color-text-muted)', fontSize: 12 }}>
        {t('background_srd_limit_note')}
      </p>
    </section>
  );
}
