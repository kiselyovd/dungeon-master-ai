import { useTranslation } from 'react-i18next';
import type { Compendium } from '../../api/srd';
import { useStore } from '../../state/useStore';
import { Field } from '../../ui/Field';
import { Select } from '../../ui/Select';
import { computeLiveSheet } from './computeLiveSheet';

export function DefenseControls({ compendium }: { compendium: Compendium }) {
  const { t, i18n } = useTranslation('wizard');
  const draft = useStore((state) => state.charCreation);
  const defense = computeLiveSheet(draft, compendium).defense;
  if (!defense) return null;
  const options = [
    { value: '', label: t('defense_unarmored') },
    ...defense.availableArmor.map((armor) => ({
      value: armor.id,
      label: i18n.language === 'ru' ? armor.name_ru : armor.name_en,
    })),
  ];
  if (defense.choice.armor_id && !options.some((item) => item.value === defense.choice.armor_id))
    options.push({ value: defense.choice.armor_id, label: t('defense_unavailable') });
  return (
    <div>
      <p>{t('defense_help')}</p>
      <Field label={t('defense_armor')}>
        {({ id }) => (
          <Select
            id={id}
            value={defense.choice.armor_id ?? ''}
            options={options}
            ariaLabel={t('defense_armor')}
            onChange={(value) =>
              draft.setDraftField('defenseChoice', { ...defense.choice, armor_id: value || null })
            }
          />
        )}
      </Field>
      <label>
        <input
          type="checkbox"
          checked={defense.choice.shield}
          disabled={!defense.shieldAvailable && !defense.choice.shield}
          onChange={(event) =>
            draft.setDraftField('defenseChoice', {
              ...defense.choice,
              shield: event.target.checked,
            })
          }
        />
        {t('defense_shield')}
      </label>
      <p>{t('defense_ac', { ac: defense.ac })}</p>
      {defense.speedPenaltyFt > 0 && <p>{t('defense_speed', { feet: defense.speedPenaltyFt })}</p>}
      {defense.stealthDisadvantage && <p>{t('defense_stealth')}</p>}
      {defense.untrainedArmor && <p>{t('defense_untrained_armor')}</p>}
      {defense.untrainedShield && <p>{t('defense_untrained_shield')}</p>}
      {!defense.valid && <p role="alert">{t('defense_invalid')}</p>}
    </div>
  );
}
