import type { StateCreator } from 'zustand';
import type { ChatErrorPayload } from '../api/errors';

export type ChatRole = 'user' | 'assistant' | 'system';

export interface ChatMessage {
  role: ChatRole;
  content: string;
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
          messages: [...s.chat.messages, { role: 'user', content }],
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
          messages: [...s.chat.messages, { role: 'assistant', content: current }],
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
