import { type ReactNode, useState } from 'react';

import styles from './CollapsibleCard.module.css';

export interface CollapsibleCardProps {
  title: string;
  chip?: string;
  defaultOpen?: boolean;
  children: ReactNode;
}

export function CollapsibleCard({
  title,
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
        <span className={styles.title}>{title}</span>
        {chip && <span className={styles.chip}>{chip}</span>}
        <span aria-hidden className={styles.chevron}>
          {open ? 'v' : '>'}
        </span>
      </button>
      {open && <div className={styles.body}>{children}</div>}
    </section>
  );
}
