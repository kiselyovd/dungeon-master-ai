import { useTranslation } from 'react-i18next';
import type { CampaignSummary } from '../../api/contracts/campaign';
import { Icons } from '../../ui/Icons';
import styles from './CampaignHome.module.css';
import { CampaignHeroCard } from './components/CampaignHeroCard';

interface CampaignHomeProps {
  campaigns: CampaignSummary[];
  loading: boolean;
  onContinue: (campaignId: string) => void;
  onNew: () => void;
}

export function CampaignHome({ campaigns, loading, onContinue, onNew }: CampaignHomeProps) {
  const { t } = useTranslation('campaigns');
  const [latest, ...archive] = campaigns;

  return (
    <main className={styles.root} data-art-direction="living-tabletop">
      <span className="dm-ambient-light" aria-hidden="true" />
      <span className="dm-ambient-dust" aria-hidden="true" />
      <div className={styles.content}>
        <header className={styles.intro}>
          <span className={styles.eyebrow}>{t('library_eyebrow')}</span>
          <h1>{t('home_title')}</h1>
          <p>{t('home_subtitle')}</p>
        </header>

        {latest ? (
          <CampaignHeroCard
            campaign={latest}
            continueLabel={t('continue_campaign')}
            revisionLabel={t('revision')}
            onContinue={onContinue}
          />
        ) : (
          <section className={styles.empty} aria-live="polite">
            <Icons.BookOpen size={28} />
            <h2>{loading ? t('loading') : t('empty_title')}</h2>
            <p>{loading ? t('loading_hint') : t('empty_body')}</p>
          </section>
        )}

        <button type="button" className={styles.newAction} onClick={onNew}>
          <Icons.Plus size={16} />
          {t('new_campaign')}
        </button>

        {archive.length > 0 && (
          <section className={styles.archive} aria-labelledby="campaign-archive-title">
            <div className={styles.archiveHeading}>
              <h2 id="campaign-archive-title">{t('archive_title')}</h2>
              <span>{archive.length}</span>
            </div>
            <div className={styles.archiveList}>
              {archive.map((campaign) => (
                <button
                  key={campaign.id}
                  type="button"
                  className={styles.archiveRow}
                  onClick={() => onContinue(campaign.id)}
                >
                  <Icons.Book size={15} />
                  <span>
                    <strong>{campaign.title}</strong>
                    <small>{campaign.premise}</small>
                  </span>
                  <span className={styles.archiveRevision}>
                    {t('revision')} {campaign.revision}
                  </span>
                  <Icons.ChevronRight size={14} />
                </button>
              ))}
            </div>
          </section>
        )}
      </div>
    </main>
  );
}
