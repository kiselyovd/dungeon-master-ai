import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import styles from './ReasoningPill.module.css';

export interface ReasoningPillProps {
  /** Current reasoning text (may be partial while streaming). */
  text: string;
  /** Whether the LLM is still producing reasoning content. */
  isStreaming: boolean;
  /** Optional override; falls back to ceil(text.length / 4) estimate. */
  totalTokens?: number;
}

function estimateTokens(text: string): number {
  return Math.ceil(text.length / 4);
}

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false;
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  });
  useEffect(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return;
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const handler = (e: MediaQueryListEvent) => setReduced(e.matches);
    // Prefer the modern addEventListener API; fall back to the legacy
    // addListener/removeListener pair for older WebKitGTK (Linux Tauri target)
    // and Safari < 14.
    if (typeof mq.addEventListener === 'function') {
      mq.addEventListener('change', handler);
      return () => mq.removeEventListener('change', handler);
    }
    mq.addListener(handler);
    return () => mq.removeListener(handler);
  }, []);
  return reduced;
}

export function ReasoningPill({ text, isStreaming, totalTokens }: ReasoningPillProps) {
  const { t } = useTranslation('chat');
  const [expanded, setExpanded] = useState(false);
  const reducedMotion = usePrefersReducedMotion();
  const tokens = totalTokens ?? estimateTokens(text);

  if (isStreaming) {
    return (
      <div
        className={`${styles.reasoningPill} ${styles.thinkingText} ${styles.noDropcap}`}
        data-reduced-motion={reducedMotion ? 'true' : 'false'}
        data-testid="reasoning-thinking"
      >
        {t('reasoning.thinking_with_tokens', { tokens })}
      </div>
    );
  }

  return (
    <div
      className={`${styles.reasoningPill} ${styles.noDropcap}`}
      data-reduced-motion={reducedMotion ? 'true' : 'false'}
    >
      <button
        type="button"
        className={styles.summaryButton}
        aria-expanded={expanded}
        onClick={() => setExpanded((v) => !v)}
      >
        {t('reasoning.collapsed_label', { tokens })}
        <span aria-hidden className={styles.chevron}>
          {expanded ? 'v' : '>'}
        </span>
      </button>
      {expanded && (
        <div className={`${styles.body} ${styles.bodyText}`} data-testid="reasoning-body">
          {text}
        </div>
      )}
    </div>
  );
}
