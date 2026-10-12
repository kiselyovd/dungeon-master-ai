import { useTranslation } from 'react-i18next';
import { portraitChoices } from '../../assets/visualCatalog';
import styles from './PortraitLibrary.module.css';

export function PortraitLibrary({
  heroClass,
  selected,
  onSelect,
  disabled = false,
}: {
  heroClass: string | null;
  selected: string | null | undefined;
  onSelect: (id: string) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation('wizard');
  const choices = portraitChoices(heroClass);
  return (
    <section className={styles.library}>
      <h3>{t('portrait_library')}</h3>
      <div className={styles.choices} role="radiogroup" aria-label={t('portrait_library')}>
        {choices.map((asset, index) => (
          <label key={asset.id} className={styles.choice} data-asset-id={asset.id}>
            <input
              type="radio"
              name="portrait-library"
              disabled={disabled}
              checked={selected === asset.id}
              onChange={() => onSelect(asset.id)}
            />
            <img src={asset.uri} alt="" loading="lazy" />
            <span>{t('portrait_variant', { number: index + 1 })}</span>
          </label>
        ))}
      </div>
    </section>
  );
}
