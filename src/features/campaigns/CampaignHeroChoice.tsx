import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { portraitChoices } from '../../assets/visualCatalog';
import { HERO_PORTRAIT } from '../../lib/heroPortraits';
import type { HeroClassId } from '../../state/pc';
import { useStore } from '../../state/useStore';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import styles from './CampaignCreate.module.css';
import { preparedHero } from './model/preparedHero';

const classes: HeroClassId[] = ['fighter', 'rogue', 'wizard', 'cleric'];
export function CampaignHeroChoice({
  campaignId,
  pending,
  onCommit,
  onCustom,
  onBack,
}: {
  campaignId: string;
  pending: boolean;
  onCommit: (character: Record<string, unknown>) => Promise<void>;
  onCustom: () => void;
  onBack: () => void;
}) {
  const { t } = useTranslation('campaigns');
  const { t: tc } = useTranslation('character');
  const bind = useStore((state) => state.charCreation.bindScope);
  const setField = useStore((state) => state.charCreation.setDraftField);
  const draft = useStore((state) => state.charCreation);
  const selected = classes.includes(draft.classId as HeroClassId)
    ? (draft.classId as HeroClassId)
    : 'fighter';
  const [error, setError] = useState(false);
  useEffect(() => {
    bind({ kind: 'campaign', campaignId, characterId: null });
  }, [bind, campaignId]);
  const scoped =
    draft.draftScope?.kind === 'campaign' && draft.draftScope.campaignId === campaignId;
  async function commit() {
    setError(false);
    try {
      const portrait =
        portraitChoices(selected).find((asset) => asset.version === 2) ??
        portraitChoices(selected)[0];
      await onCommit({
        ...preparedHero(selected),
        name: draft.name.trim() || tc(`class_${selected}`),
        portraitUrl: draft.portraitUrl,
        portraitMediaRef: draft.portraitMediaRef ?? null,
        portraitAssetId:
          draft.portraitAssetId ?? (draft.portraitMediaRef ? null : (portrait?.id ?? null)),
      });
    } catch {
      setError(true);
    }
  }
  if (!scoped) return <main className={styles.root} aria-busy="true" />;
  return (
    <main className={styles.root}>
      <div className={styles.sheet}>
        <header className={styles.header}>
          <h1>{t('hero_choice_title')}</h1>
        </header>
        <section className={styles.stage}>
          <div
            className={`dm-hero-cards ${styles.heroCards}`}
            role="radiogroup"
            aria-label={t('hero_choice_title')}
          >
            {classes.map((classId) => (
              <label key={classId} className="dm-hero-card">
                <input
                  type="radio"
                  name="prepared-hero"
                  className={styles.pathInput}
                  checked={selected === classId}
                  disabled={pending}
                  onChange={() => setField('classId', classId)}
                />
                <div className="dm-hero-card-portrait">
                  <img
                    src={
                      portraitChoices(classId).find((asset) => asset.version === 2)?.uri ??
                      HERO_PORTRAIT[classId]
                    }
                    alt=""
                  />
                </div>
                <div className="dm-hero-card-body">
                  <strong>{tc(`class_${classId}`)}</strong>
                  <span>
                    {t('hero_stats', {
                      hp: preparedHero(classId).hp,
                      ac: preparedHero(classId).ac,
                    })}
                  </span>
                </div>
              </label>
            ))}
          </div>
          <Field label={t('hero_name')}>
            {(props) => (
              <input
                {...props}
                value={draft.name}
                onChange={(event) => setField('name', event.target.value)}
                disabled={pending}
              />
            )}
          </Field>
          {error && <p role="alert">{t('hero_commit_failed')}</p>}
        </section>
        <footer className={styles.actions}>
          <Button onClick={onBack} disabled={pending}>
            {t('back')}
          </Button>
          <Button onClick={onCustom} disabled={pending}>
            {t('hero_custom')}
          </Button>
          <Button variant="primary" onClick={() => void commit()} disabled={pending}>
            {t('enter_scene')}
          </Button>
        </footer>
      </div>
    </main>
  );
}
