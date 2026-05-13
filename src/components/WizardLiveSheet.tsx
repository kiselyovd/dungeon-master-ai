import { useTranslation } from 'react-i18next';
import type { LiveSheet } from './wizard/computeLiveSheet';

export interface WizardLiveSheetProps {
  sheet: LiveSheet;
}

export function WizardLiveSheet({ sheet }: WizardLiveSheetProps) {
  const { t } = useTranslation('wizard');

  if (sheet.placeholder) {
    return (
      <aside className="dm-wizard-live" aria-label={t('live_sheet_label')}>
        <div className="dm-wizard-live-placeholder">{t('pick_a_class_to_begin')}</div>
      </aside>
    );
  }

  return (
    <aside className="dm-wizard-live" aria-label={t('live_sheet_label')}>
      <header className="dm-wizard-live-identity">
        <div className="dm-wizard-live-name">{sheet.name ?? t('placeholder_hero')}</div>
        <div className="dm-wizard-live-class">
          {sheet.className} {sheet.level} - {sheet.raceName ?? t('no_race')}
        </div>
      </header>
      <section className="dm-wizard-live-combat">
        <div className="dm-wizard-live-stat">
          <span className="dm-wizard-live-stat-label">{t('hp')}</span>
          <span className="dm-wizard-live-stat-value">
            {sheet.hp ?? '-'} / {sheet.hpMax ?? '-'}
          </span>
        </div>
        <div className="dm-wizard-live-stat">
          <span className="dm-wizard-live-stat-label">{t('ac')}</span>
          <span className="dm-wizard-live-stat-value">{sheet.ac ?? '-'}</span>
        </div>
        <div className="dm-wizard-live-stat">
          <span className="dm-wizard-live-stat-label">{t('init')}</span>
          <span className="dm-wizard-live-stat-value">{formatMod(sheet.initiative)}</span>
        </div>
        <div className="dm-wizard-live-stat">
          <span className="dm-wizard-live-stat-label">{t('speed')}</span>
          <span className="dm-wizard-live-stat-value">
            {sheet.speedFt !== null ? `${sheet.speedFt} ${t('ft')}` : '-'}
          </span>
        </div>
      </section>
      <section className="dm-wizard-live-abilities">
        {(['str', 'dex', 'con', 'int', 'wis', 'cha'] as const).map((k) => (
          <div key={k} className="dm-wizard-live-ability-row">
            <span>{t(`ability_${k}`)}</span>
            <span>{sheet.abilities[k].score}</span>
            <span>({formatMod(sheet.abilities[k].mod)})</span>
          </div>
        ))}
      </section>
      <section className="dm-wizard-live-inventory">
        <div className="dm-wizard-live-section-label">{t('inventory')}</div>
        <ul>
          {sheet.inventoryPreview.map((it) => (
            <li key={it.id}>
              {it.name}
              {it.count > 1 ? ` x${it.count}` : ''}
            </li>
          ))}
        </ul>
        {sheet.inventoryOverflow > 0 && (
          <div className="dm-wizard-live-inventory-more">+{sheet.inventoryOverflow}</div>
        )}
      </section>
    </aside>
  );
}

function formatMod(value: number | null): string {
  if (value === null) return '-';
  return value >= 0 ? `+${value}` : `${value}`;
}
