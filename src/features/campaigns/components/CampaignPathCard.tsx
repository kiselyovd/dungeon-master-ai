import type { ReactNode } from 'react';
import styles from '../CampaignCreate.module.css';

interface CampaignPathCardProps {
  checked: boolean;
  title: string;
  description: string;
  icon: ReactNode;
  onSelect: () => void;
}

export function CampaignPathCard({
  checked,
  title,
  description,
  icon,
  onSelect,
}: CampaignPathCardProps) {
  return (
    <label className={styles.pathCard} data-selected={checked}>
      <input
        type="radio"
        name="campaign-source"
        checked={checked}
        className={styles.pathInput}
        onChange={onSelect}
      />
      <span className={styles.pathIcon}>{icon}</span>
      <span className={styles.pathCopy}>
        <strong>{title}</strong>
        <span className={styles.pathDescription}>{description}</span>
      </span>
    </label>
  );
}
