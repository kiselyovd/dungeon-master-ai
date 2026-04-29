import type { ReactNode } from 'react';
import type { ChatRole } from '../state/chat';
import styles from './MessageBubble.module.css';

interface MessageBubbleProps {
  chatRole: ChatRole;
  /** When true, render with a "live" treatment (italic + accent border). */
  streaming?: boolean;
  children: ReactNode;
}

/**
 * Single chat-history bubble.
 *
 * Visual variants:
 * - user: gold-tinted background, right-aligned.
 * - assistant: raised background, left-aligned.
 * - assistant + streaming: italic with an accent border so the user can tell
 *   the live response apart from finalised history.
 *
 * The bubble's outer container is a presentational div - aria-live for the
 * streaming case is set at the chat-list level (in ChatPanel) so screen
 * readers receive a single combined announcement, not per-bubble noise.
 */
export function MessageBubble({ chatRole, streaming = false, children }: MessageBubbleProps) {
  return (
    <div
      className={styles.bubble}
      data-role={chatRole}
      data-streaming={streaming ? 'true' : undefined}
    >
      {children}
    </div>
  );
}
