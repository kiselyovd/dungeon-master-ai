import type { ReactNode } from 'react';
import type { ChatRole } from '../state/chat';

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
  const isUser = chatRole === 'user';
  return (
    <div
      data-role={chatRole}
      data-streaming={streaming || undefined}
      style={{
        alignSelf: isUser ? 'flex-end' : 'flex-start',
        maxWidth: '80%',
        padding: 'var(--space-3) var(--space-4)',
        borderRadius: 'var(--radius-md)',
        background: isUser ? 'var(--color-accent-soft)' : 'var(--color-bg-raised)',
        border: streaming
          ? '1px solid var(--color-accent)'
          : '1px solid var(--color-border-subtle)',
        fontStyle: streaming ? 'italic' : undefined,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word',
      }}
    >
      {children}
    </div>
  );
}
