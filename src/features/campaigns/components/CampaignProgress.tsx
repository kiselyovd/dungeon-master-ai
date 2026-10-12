import { useTranslation } from 'react-i18next';
import type { CampaignSummary } from '../../../api/contracts/campaign';
import styles from '../CampaignHome.module.css';

export function CampaignProgress({
  campaign,
  visitedAt,
}: {
  campaign: CampaignSummary;
  visitedAt?: string | undefined;
}) {
  const { t, i18n } = useTranslation('campaigns');
  const { t: ta } = useTranslation('adventure');
  const date = (value: string) => {
    const parsed = new Date(value);
    return Number.isFinite(parsed.getTime())
      ? new Intl.DateTimeFormat(i18n.language === 'ru' ? 'ru-RU' : 'en-GB', {
          dateStyle: 'medium',
          timeStyle: 'short',
        }).format(parsed)
      : null;
  };
  const location = campaign.current_location;
  const locationLabel =
    location?.kind === 'authored'
      ? ta(`locations.${location.location}`)
      : location?.kind === 'scene'
        ? location.title
        : null;
  const saved = date(campaign.updated_at);
  const visited = visitedAt ? date(visitedAt) : null;
  return (
    <span className={styles.progress}>
      {locationLabel && <span className={styles.location}>{locationLabel}</span>}
      {saved && <time dateTime={campaign.updated_at}>{t('saved_at', { date: saved })}</time>}
      {visited && (
        <time dateTime={visitedAt} title={t('opened_on_device', { date: visited })}>
          {t('last_opened', { date: visited })}
        </time>
      )}
    </span>
  );
}
