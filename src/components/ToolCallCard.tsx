import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ToolLogEntry } from '../state/toolLog';
import styles from './ToolCallCard.module.css';

interface Props {
  entry: ToolLogEntry;
}

/**
 * Tool-call card in the chat history.
 *
 * IMPORTANT: parent must render with `key={entry.id}` to ensure each tool call
 * gets a fresh component instance. The internal state (settled/flashing flags,
 * cycling interval) is reset when entry.id changes, but reusing the same
 * instance for unrelated entries can briefly bleed state during a render.
 *
 * Settle animation (design delta B):
 * - When `entry.result` is null, cycle random digits at 100ms interval.
 * - When `entry.result` arrives (not null), snap to real value + add gold flash.
 * - Animation fires only when result actually arrives, never ahead of truth.
 */
export function ToolCallCard({ entry }: Props) {
  const { toolName, args, result, isError, round } = entry;
  const pending = result === null;
  const { t } = useTranslation('agent');

  const [displayResult, setDisplayResult] = useState<string>('...');
  const [settled, setSettled] = useState(false);
  const [flashing, setFlashing] = useState(false);

  useEffect(() => {
    if (pending) {
      // New pending state - reset flags from any prior settled state.
      setSettled(false);
      setFlashing(false);
      setDisplayResult('...');
      const interval = setInterval(() => {
        const n = Math.floor(Math.random() * 20) + 1;
        setDisplayResult(String(n));
      }, 100);
      return () => clearInterval(interval);
    }

    setDisplayResult(JSON.stringify(result, null, 2));
    setSettled(true);
    setFlashing(true);
    const flashTimer = setTimeout(() => setFlashing(false), 600);
    return () => clearTimeout(flashTimer);
  }, [pending, result]);

  const statusKey = pending ? 'tool_pending' : isError ? 'tool_error' : 'tool_success';
  const statusLabel = pending ? 'pending' : isError ? 'error' : 'success';

  return (
    <div
      className={`${styles.card} ${isError ? styles.cardError : ''} ${flashing ? styles.cardFlash : ''}`}
      data-status={statusLabel}
    >
      <div className={styles.header}>
        <span className={styles.toolName}>{toolName}</span>
        <span className={`${styles.statusBadge} ${styles[`status_${statusLabel}`]}`}>
          {t(statusKey)}
        </span>
        <span className={styles.round}>{t('round_label', { round })}</span>
      </div>
      <div className={styles.body}>
        <div className={styles.section}>
          <span className={styles.label}>args</span>
          <pre className={styles.code}>{JSON.stringify(args, null, 2)}</pre>
        </div>
        <div className={styles.section}>
          <span className={styles.label}>result</span>
          <pre
            className={`${styles.code} ${pending ? styles.cycling : ''} ${settled ? styles.settled : ''}`}
          >
            {displayResult}
          </pre>
        </div>
      </div>
    </div>
  );
}
