import { useTranslation } from 'react-i18next';
import type { CampaignSummary } from '../../api/contracts/campaign';
import { useStore } from '../../state/useStore';
import { Icons } from '../../ui/Icons';
import styles from './CampaignHome.module.css';
import { CampaignHeroCard } from './components/CampaignHeroCard';
import { CampaignProgress } from './components/CampaignProgress';

interface CampaignHomeProps {
  errorCode?: string | null;
  onRetry?: () => void;
  campaigns: CampaignSummary[];
  loading: boolean;
  onContinue: (campaignId: string) => void;
  onNew: () => void;
}

export function CampaignHome({
  campaigns,
  loading,
  onContinue,
  onNew,
  errorCode,
  onRetry,
}: CampaignHomeProps) {
  const { t } = useTranslation('campaigns');
  const visits = useStore((state) => state.settings.campaignVisits);
  const [latest, ...archive] = campaigns;

  return (
    <main className={styles.root} data-art-direction="living-tabletop">
      <span className="dm-ambient-light" aria-hidden="true" />
      <span className="dm-ambient-dust" aria-hidden="true" />
      <div className={styles.content}>
        <header className={styles.intro}>
          <h1>{t('home_title')}</h1>
        </header>
        {errorCode && (
          <section role="alert" className={styles.empty}>
            <p>{t('load_failed')}</p>
            {onRetry && (
              <button
                type="button"
                className={styles.primaryAction}
                disabled={loading}
                onClick={onRetry}
              >
                {t('retry')}
              </button>
            )}
          </section>
        )}

        {latest ? (
          <CampaignHeroCard
            campaign={latest}
            continueLabel={t('continue_campaign')}
            visitedAt={visits[latest.id]}
            onContinue={onContinue}
            disabled={loading}
          />
        ) : (
          <section className={styles.empty} aria-live="polite">
            <Icons.BookOpen size={28} />
            <h2>{loading ? t('loading') : t('empty_title')}</h2>
          </section>
        )}

        <button type="button" className={styles.newAction} onClick={onNew} disabled={loading}>
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
                  disabled={loading}
                  onClick={() => onContinue(campaign.id)}
                >
                  <Icons.Book size={15} />
                  <span>
                    <strong>{campaign.title}</strong>
                    <small>{campaign.premise}</small>
                  </span>
                  <CampaignProgress campaign={campaign} visitedAt={visits[campaign.id]} />
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
