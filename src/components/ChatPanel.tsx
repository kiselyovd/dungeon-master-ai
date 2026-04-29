import { type KeyboardEvent, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { ChatErrorCode } from '../api/errors';
import { useChat } from '../hooks/useChat';
import { MessageBubble } from './MessageBubble';

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
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        background: 'var(--color-bg-base)',
      }}
    >
      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: 'var(--space-4)',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--space-3)',
        }}
      >
        {messages.map((m) => (
          <MessageBubble key={m.id} chatRole={m.role}>
            {m.content}
          </MessageBubble>
        ))}
        {streamingAssistant !== null && (
          <div aria-live="polite" style={{ display: 'contents' }}>
            <MessageBubble chatRole="assistant" streaming>
              {streamingAssistant}
            </MessageBubble>
          </div>
        )}
        {lastError !== null && (
          <div
            role="alert"
            style={{
              padding: 'var(--space-3)',
              borderLeft: '3px solid var(--color-danger)',
              background: 'rgba(196, 68, 68, 0.08)',
              fontSize: 'var(--text-sm)',
            }}
          >
            {tErrors(lastError.code as ChatErrorCode, { message: lastError.message })}
          </div>
        )}
      </div>
      <div
        style={{
          borderTop: '1px solid var(--color-border-strong)',
          padding: 'var(--space-3)',
          display: 'flex',
          gap: 'var(--space-2)',
        }}
      >
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={t('placeholder')}
          rows={2}
          style={{ flex: 1, resize: 'none' }}
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
