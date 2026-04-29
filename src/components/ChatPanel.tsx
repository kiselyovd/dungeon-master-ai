import { type KeyboardEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ChatErrorCode } from '../api/errors';
import { useChat } from '../hooks/useChat';
import styles from './ChatPanel.module.css';
import { MessageBubble } from './MessageBubble';
import { TypingIndicator } from './TypingIndicator';

export function ChatPanel() {
  const { t } = useTranslation('chat');
  const { t: tErrors } = useTranslation('errors');
  const { messages, streamingAssistant, isStreaming, lastError, send, cancel } = useChat();
  const [draft, setDraft] = useState('');

  const canSend = !isStreaming && draft.trim().length > 0;

  const onSend = async () => {
    if (!canSend) return;
    const text = draft;
    setDraft('');
    await send(text);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void onSend();
    } else if (e.key === 'Escape' && isStreaming) {
      e.preventDefault();
      cancel();
    }
  };

  // Window-level ESC also aborts an in-flight stream so the user can hit it
  // even when focus has wandered out of the textarea.
  useEffect(() => {
    if (!isStreaming) return;
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === 'Escape') cancel();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isStreaming, cancel]);

  return (
    <div className={styles.panel}>
      <div className={styles.history}>
        {messages.map((m) => (
          <MessageBubble key={m.id} chatRole={m.role}>
            {m.content}
          </MessageBubble>
        ))}
        {(isStreaming || streamingAssistant !== null) && (
          <div aria-live="polite" className={styles.streamWrapper}>
            {streamingAssistant === null || streamingAssistant === '' ? (
              <div className={styles.typingRow}>
                <TypingIndicator />
              </div>
            ) : (
              <MessageBubble chatRole="assistant" streaming>
                {streamingAssistant}
              </MessageBubble>
            )}
          </div>
        )}
        {lastError !== null && (
          <div role="alert" className={styles.errorAlert}>
            {tErrors(lastError.code as ChatErrorCode, { message: lastError.message })}
          </div>
        )}
      </div>
      <div className={styles.composer}>
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={t('placeholder')}
          rows={2}
          className={styles.draft}
        />
        {isStreaming ? (
          <button type="button" onClick={cancel} aria-label={t('stop')}>
            {t('stop')}
          </button>
        ) : (
          <button type="button" onClick={() => void onSend()} disabled={!canSend}>
            {t('send')}
          </button>
        )}
      </div>
    </div>
  );
}
