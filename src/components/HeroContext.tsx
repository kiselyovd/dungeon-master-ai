import { useTranslation } from 'react-i18next';
import type { PcData } from '../state/pc';

export function HeroContext({ character }: { character: Partial<PcData> | null }) {
  const { t } = useTranslation('character');
  if (!character) return <p>{t('empty_title')}</p>;
  return (
    <section className="dm-hero-context">
      <h2>{character.name}</h2>
      {character.heroClass && (
        <p>{t(`catalog.classes.${character.heroClass}`, { defaultValue: character.heroClass })}</p>
      )}
      <dl className="dm-hero-context-vitals">
        {typeof character.hp === 'number' && typeof character.hpMax === 'number' && (
          <>
            <dt>{t('stat_hp')}</dt>
            <dd>
              {character.hp}/{character.hpMax}
            </dd>
          </>
        )}
        {typeof character.ac === 'number' && (
          <>
            <dt>{t('stat_ac')}</dt>
            <dd>{character.ac}</dd>
          </>
        )}
        {typeof character.speedFt === 'number' && (
          <>
            <dt>{t('stat_speed')}</dt>
            <dd>{t('stat_speed_value', { ft: character.speedFt })}</dd>
          </>
        )}
      </dl>
      {character.abilities && (
        <section>
          <h3>{t('section_abilities')}</h3>
          <dl className="dm-hero-context-vitals">
            {(['str', 'dex', 'con', 'int', 'wis', 'cha'] as const)
              .filter((key) => typeof character.abilities?.[key] === 'number')
              .map((key) => (
                <div key={key}>
                  <dt>{t(`ability_${key}`)}</dt>
                  <dd>{character.abilities?.[key]}</dd>
                </div>
              ))}
          </dl>
        </section>
      )}
      {character.inventory && (
        <section>
          <h3>{t('section_inventory')}</h3>
          <ul>
            {character.inventory.map((item) => (
              <li key={item.id}>
                {item.name} × {item.count}
              </li>
            ))}
          </ul>
        </section>
      )}
      {(['backstory', 'ideals', 'bonds', 'flaws'] as const).map(
        (key) =>
          character[key] && (
            <section key={key}>
              <h3>{t(`context_${key}`)}</h3>
              <p>{character[key]}</p>
            </section>
          ),
      )}
    </section>
  );
}
