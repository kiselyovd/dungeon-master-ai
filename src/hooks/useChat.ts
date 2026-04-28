import { useCallback, useState } from 'react';
import { streamChat } from '../api/chat';
import { useStore } from '../state/useStore';

export function useChat() {
  const messages = useStore((s) => s.chat.messages);
  const streamingAssistant = useStore((s) => s.chat.streamingAssistant);
  const appendUser = useStore((s) => s.chat.appendUser);
  const appendDelta = useStore((s) => s.chat.appendAssistantDelta);
  const finalize = useStore((s) => s.chat.finalizeAssistant);

  const [isStreaming, setStreaming] = useState(false);
  const [lastError, setLastError] = useState<{ code: string; message: string } | null>(null);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim()) return;
      appendUser(text);
      setStreaming(true);
      setLastError(null);

      const baseMessages = [...useStore.getState().chat.messages];
      try {
        await streamChat({
          messages: baseMessages,
          onTextDelta: appendDelta,
          onDone: () => {
            finalize();
            setStreaming(false);
          },
          onError: (err) => {
            setLastError(err);
            finalize();
            setStreaming(false);
          },
        });
      } catch (e) {
        setLastError({ code: 'unknown', message: String(e) });
        setStreaming(false);
      }
    },
    [appendUser, appendDelta, finalize],
  );

  return { messages, streamingAssistant, isStreaming, lastError, send };
}
