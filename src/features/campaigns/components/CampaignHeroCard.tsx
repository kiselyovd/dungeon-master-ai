import type { CampaignSummary } from '../../../api/contracts/campaign';
import { KEY_ART, KEY_ART_IDS } from '../../../assets/livingTabletop';
import { DecorativeImage } from '../../../ui/DecorativeImage';
import { Icons } from '../../../ui/Icons';
import styles from '../CampaignHome.module.css';
import { CampaignProgress } from './CampaignProgress';

interface CampaignHeroCardProps {
  disabled?: boolean;
  campaign: CampaignSummary;
  continueLabel: string;
  visitedAt?: string | undefined;
  onContinue: (campaignId: string) => void;
}

export function CampaignHeroCard({
  campaign,
  continueLabel,
  visitedAt,
  onContinue,
  disabled,
}: CampaignHeroCardProps) {
  return (
    <article className={styles.heroCard}>
      <div className={styles.heroArt} aria-hidden="true">
        <DecorativeImage src={KEY_ART.vttEmpty} assetId={KEY_ART_IDS.vttEmpty} />
      </div>
      <div className={styles.heroBody}>
        <div className={styles.heroCopy}>
          <h2 className={styles.heroTitle}>{campaign.title}</h2>
          <p>{campaign.premise}</p>
          <CampaignProgress campaign={campaign} visitedAt={visitedAt} />
        </div>
        <button
          type="button"
          className={styles.primaryAction}
          disabled={disabled}
          onClick={() => onContinue(campaign.id)}
        >
          <Icons.BookOpen size={16} />
          {continueLabel}
          <Icons.ChevronRight size={15} />
        </button>
      </div>
    </article>
  );
}
