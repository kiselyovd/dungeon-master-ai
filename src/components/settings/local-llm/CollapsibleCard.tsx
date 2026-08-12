import { type ReactNode, useState } from 'react';
import { Icons } from '../../../ui/Icons';

import styles from './CollapsibleCard.module.css';

export interface CollapsibleCardProps {
  title: string;
  icon?: ReactNode;
  chip?: string;
  defaultOpen?: boolean;
  children: ReactNode;
}

export function CollapsibleCard({
  title,
  icon,
  chip,
  defaultOpen = false,
  children,
}: CollapsibleCardProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <section className={styles.card}>
      <button
        type="button"
        className={styles.header}
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {icon && (
          <span aria-hidden className={styles.icon}>
            {icon}
          </span>
        )}
        <span className={styles.title}>{title}</span>
        {chip && <span className={styles.chip}>{chip}</span>}
        <Icons.ChevronDown aria-hidden className={styles.chevron ?? ''} size={16} />
      </button>
      {open && <div className={styles.body}>{children}</div>}
    </section>
  );
}
