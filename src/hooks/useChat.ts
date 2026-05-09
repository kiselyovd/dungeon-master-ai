import { useCallback } from 'react';
import { streamChat } from '../api/chat';
import { ChatError } from '../api/errors';
import { stripDataUrlPrefix } from '../lib/fileToDataUrl';
import type { MessagePart, StagedImage } from '../state/chat';
import { useStore } from '../state/useStore';

/**
 * Send orchestrator. Wires user input -> store actions -> streamChat -> store
 * deltas. The streaming/error state lives in the chat slice (so a global Stop
 * button or shortcut handler can subscribe), this hook just reads/triggers.
 */
export function useChat() {
  const messages = useStore((s) => s.chat.messages);
  const streamingAssistant = useStore((s) => s.chat.streamingAssistant);
  const isStreaming = useStore((s) => s.chat.isStreaming);
  const lastError = useStore((s) => s.chat.lastError);

  const appendUser = useStore((s) => s.chat.appendUser);
  const appendDelta = useStore((s) => s.chat.appendAssistantDelta);
  const finalize = useStore((s) => s.chat.finalizeAssistant);
  const beginStream = useStore((s) => s.chat.beginStream);
  const endStream = useStore((s) => s.chat.endStream);
  const setError = useStore((s) => s.chat.setError);
  const abort = useStore((s) => s.chat.abort);

  const send = useCallback(
    async (text: string, images: StagedImage[] = []) => {
      const trimmed = text.trim();
      if (!trimmed && images.length === 0) return;
      if (useStore.getState().chat.isStreaming) return;

      const parts: MessagePart[] = [];
      if (trimmed) parts.push({ type: 'text', text });
      for (const img of images) {
        parts.push({
          type: 'image',
          mime: img.mime,
          data_b64: stripDataUrlPrefix(img.dataUrl),
          name: img.name ?? null,
        });
      }
      appendUser(text, images.length > 0 ? parts : undefined);

      const controller = new AbortController();
      beginStream(controller);

      const baseMessages = useStore.getState().chat.messages;
      try {
        await streamChat({
          messages: baseMessages,
          onTextDelta: appendDelta,
          signal: controller.signal,
        });
      } catch (e) {
        setError(ChatError.from(e).toPayload());
      } finally {
        finalize();
        endStream();
      }
    },
    [appendUser, appendDelta, finalize, beginStream, endStream, setError],
  );

  return {
    messages,
    streamingAssistant,
    isStreaming,
    lastError,
    send,
    cancel: abort,
  };
}
