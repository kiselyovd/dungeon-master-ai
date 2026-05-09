import type { StateCreator } from 'zustand';
import type { ChatErrorPayload } from '../api/errors';

export type ChatRole = 'user' | 'assistant' | 'system';

/**
 * A single part of a multimodal user message. Mirrors the backend's
 * `MessagePart` enum: text or image-with-base64.
 */
export type MessagePart =
  | { type: 'text'; text: string }
  | { type: 'image'; mime: string; data_b64: string; name?: string | null };

/**
 * Rendered chat message. `id` is frontend-only (React keys + dedupe).
 * `content` is the canonical text body; for user messages that include
 * images, `parts` carries the full multimodal payload. Components that only
 * need text can read `content`; the composer + bubble rendering branch on
 * `parts` when present.
 */
export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  parts?: MessagePart[];
}

/** A staged image attached to the composer before the user sends. */
export interface StagedImage {
  /** MIME type, e.g. `image/png`. */
  mime: string;
  /** Full data URL including `data:image/...;base64,` prefix. */
  dataUrl: string;
  /** Original filename, if any. */
  name?: string;
  sizeBytes: number;
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

    appendUser: (content: string, parts?: MessagePart[]) => void;
    appendAssistantDelta: (delta: string) => void;
    finalizeAssistant: () => void;
    /** Replace the entire history (used after loading from /sessions/:id/messages). */
    setMessages: (messages: ChatMessage[]) => void;
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

    appendUser: (content, parts) =>
      set((s) => {
        const msg: ChatMessage = { id: newMessageId(), role: 'user', content };
        if (parts && parts.length > 0) msg.parts = parts;
        return {
          chat: {
            ...s.chat,
            messages: [...s.chat.messages, msg],
          },
        };
      }),

    setMessages: (messages) =>
      set((s) => ({
        chat: { ...s.chat, messages },
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
