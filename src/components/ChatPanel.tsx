import { type KeyboardEvent, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useChat } from '../hooks/useChat';

export function ChatPanel() {
  const { t } = useTranslation('chat');
  const { messages, streamingAssistant, isStreaming, lastError, send } = useChat();
  const [draft, setDraft] = useState('');

  const onSend = async () => {
    if (!draft.trim() || isStreaming) return;
    const text = draft;
    setDraft('');
    await send(text);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void onSend();
    }
  };

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
        {messages.map((m, i) => (
          <div
            key={i}
            style={{
              alignSelf: m.role === 'user' ? 'flex-end' : 'flex-start',
              maxWidth: '80%',
              padding: 'var(--space-3) var(--space-4)',
              borderRadius: 'var(--radius-md)',
              background: m.role === 'user' ? 'var(--color-accent-soft)' : 'var(--color-bg-raised)',
              border: '1px solid var(--color-border-subtle)',
            }}
          >
            {m.content}
          </div>
        ))}
        {streamingAssistant !== null && (
          <div
            style={{
              alignSelf: 'flex-start',
              maxWidth: '80%',
              padding: 'var(--space-3) var(--space-4)',
              borderRadius: 'var(--radius-md)',
              background: 'var(--color-bg-raised)',
              border: '1px solid var(--color-accent)',
              fontStyle: 'italic',
            }}
          >
            {streamingAssistant}
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
            {lastError.code}: {lastError.message}
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
        <button type="button" onClick={() => void onSend()} disabled={!draft.trim() || isStreaming}>
          {isStreaming ? t('thinking') : t('send')}
        </button>
      </div>
    </div>
  );
}
