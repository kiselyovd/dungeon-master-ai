import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './ReasoningPill.module.css';

interface ReasoningPillProps {
  thinkingText: string;
  /** Whether the LLM is currently producing thinking content. Shows an animated indicator. */
  streaming?: boolean;
}

export function ReasoningPill({ thinkingText, streaming = false }: ReasoningPillProps) {
  const [expanded, setExpanded] = useState(false);
  const { t } = useTranslation('chat');

  if (!thinkingText && !streaming) return null;

  return (
    <div className={styles.container} data-testid="reasoning-pill">
      <button
        type="button"
        className={styles.pill}
        aria-expanded={expanded}
        aria-controls="reasoning-pill-content"
        onClick={() => setExpanded((v) => !v)}
      >
        <span className={styles.icon} aria-hidden>
          {streaming ? '...' : '*'}
        </span>
        <span>{streaming ? t('reasoning_streaming_label') : t('reasoning_thinking_label')}</span>
      </button>
      {expanded && thinkingText && (
        <div id="reasoning-pill-content" className={styles.content} aria-live="polite">
          {thinkingText}
        </div>
      )}
    </div>
  );
}
