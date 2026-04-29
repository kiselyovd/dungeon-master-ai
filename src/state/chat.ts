import type { StateCreator } from 'zustand';
import type { ChatErrorPayload } from '../api/errors';

export type ChatRole = 'user' | 'assistant' | 'system';

export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
}

function newMessageId(): string {
  // crypto.randomUUID is in the runtime web spec and works under both Tauri
  // (recent WebViews) and jsdom; falls back to a millisecond+random hybrid
  // for any odd environment that lacks it.
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

export interface ChatSlice {
  chat: {
    messages: ChatMessage[];
    streamingAssistant: string | null;
    isStreaming: boolean;
    lastError: ChatErrorPayload | null;
    abortController: AbortController | null;

    appendUser: (content: string) => void;
    appendAssistantDelta: (delta: string) => void;
    finalizeAssistant: () => void;
    reset: () => void;

    /** Mark the start of a stream and store the controller used to abort it. */
    beginStream: (controller: AbortController) => void;
    /** Mark the stream ended (success or error). Clears the controller. */
    endStream: () => void;
    /** Trigger an abort on the active controller, if any. */
    abort: () => void;
    setError: (err: ChatErrorPayload | null) => void;
  };
}

export const createChatSlice: StateCreator<ChatSlice, [], [], ChatSlice> = (set, get) => ({
  chat: {
    messages: [],
    streamingAssistant: null,
    isStreaming: false,
    lastError: null,
    abortController: null,

    appendUser: (content) =>
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [...s.chat.messages, { id: newMessageId(), role: 'user', content }],
        },
      })),

    appendAssistantDelta: (delta) => {
      if (delta.length === 0) return;
      set((s) => ({
        chat: {
          ...s.chat,
          streamingAssistant: (s.chat.streamingAssistant ?? '') + delta,
        },
      }));
    },

    finalizeAssistant: () => {
      const current = get().chat.streamingAssistant;
      if (current === null || current.length === 0) {
        if (current === '') set((s) => ({ chat: { ...s.chat, streamingAssistant: null } }));
        return;
      }
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [
            ...s.chat.messages,
            { id: newMessageId(), role: 'assistant', content: current },
          ],
          streamingAssistant: null,
        },
      }));
    },

    reset: () =>
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [],
          streamingAssistant: null,
          lastError: null,
        },
      })),

    beginStream: (controller) =>
      set((s) => ({
        chat: { ...s.chat, isStreaming: true, abortController: controller, lastError: null },
      })),

    endStream: () =>
      set((s) => ({
        chat: { ...s.chat, isStreaming: false, abortController: null },
      })),

    abort: () => {
      const controller = get().chat.abortController;
      if (controller && !controller.signal.aborted) {
        controller.abort();
      }
    },

    setError: (lastError) => set((s) => ({ chat: { ...s.chat, lastError } })),
  },
});
