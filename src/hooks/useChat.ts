import { useCallback, useState } from 'react';
import { streamChat } from '../api/chat';
import { ChatError, type ChatErrorPayload } from '../api/errors';
import { useStore } from '../state/useStore';

export function useChat() {
  const messages = useStore((s) => s.chat.messages);
  const streamingAssistant = useStore((s) => s.chat.streamingAssistant);
  const appendUser = useStore((s) => s.chat.appendUser);
  const appendDelta = useStore((s) => s.chat.appendAssistantDelta);
  const finalize = useStore((s) => s.chat.finalizeAssistant);

  const [isStreaming, setStreaming] = useState(false);
  const [lastError, setLastError] = useState<ChatErrorPayload | null>(null);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim()) return;
      appendUser(text);
      setStreaming(true);
      setLastError(null);

      const baseMessages = useStore.getState().chat.messages;
      try {
        await streamChat({ messages: baseMessages, onTextDelta: appendDelta });
      } catch (e) {
        setLastError(ChatError.from(e).toPayload());
      } finally {
        finalize();
        setStreaming(false);
      }
    },
    [appendUser, appendDelta, finalize],
  );

  return { messages, streamingAssistant, isStreaming, lastError, send };
}
