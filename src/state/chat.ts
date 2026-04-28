import type { StateCreator } from 'zustand';

export type ChatRole = 'user' | 'assistant' | 'system';

export interface ChatMessage {
  role: ChatRole;
  content: string;
}

export interface ChatSlice {
  chat: {
    messages: ChatMessage[];
    streamingAssistant: string | null;
    appendUser: (content: string) => void;
    appendAssistantDelta: (delta: string) => void;
    finalizeAssistant: () => void;
    reset: () => void;
  };
}

export const createChatSlice: StateCreator<ChatSlice, [], [], ChatSlice> = (set, get) => ({
  chat: {
    messages: [],
    streamingAssistant: null,
    appendUser: (content) =>
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [...s.chat.messages, { role: 'user', content }],
        },
      })),
    appendAssistantDelta: (delta) =>
      set((s) => ({
        chat: {
          ...s.chat,
          streamingAssistant: (s.chat.streamingAssistant ?? '') + delta,
        },
      })),
    finalizeAssistant: () => {
      const current = get().chat.streamingAssistant;
      if (current === null) return;
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [...s.chat.messages, { role: 'assistant', content: current }],
          streamingAssistant: null,
        },
      }));
    },
    reset: () => set((s) => ({ chat: { ...s.chat, messages: [], streamingAssistant: null } })),
  },
});
