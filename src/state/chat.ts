import i18next from 'i18next';
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
 *
 * `sequenceIndex` is a monotonic counter shared with `ChatStreamEvent` so
 * messages and inline tool-call cards can be merged into a single ordered list.
 */
export interface ChatMessage {
  id: string;
  role: ChatRole;
  content: string;
  parts?: MessagePart[];
  /**
   * Shared monotonic ordering index with ChatStreamEvent for merged rendering.
   * Optional for back-compat with messages loaded from the backend (via setMessages)
   * and test fixtures that predate B2; those sort to the front (treated as 0).
   */
  sequenceIndex?: number;
}

/** Status of an inline tool-call card in the chat stream. */
export type ChatStreamEventStatus = 'pending' | 'success' | 'error';

/**
 * An inline tool-call event emitted during a streaming agent turn.
 * Rendered as a ToolCallCard between message bubbles, ordered by sequenceIndex.
 */
export interface ChatStreamEvent {
  id: string;
  toolName: string;
  sequenceIndex: number;
  status: ChatStreamEventStatus;
  args: unknown;
  result: unknown | null;
  isError: boolean;
  round: number;
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
    /** Inline tool-call events emitted during the current (or most recent) stream. */
    chatStreamEvents: ChatStreamEvent[];
    /** Monotonic sequence counter shared between messages and stream events. */
    _nextSeq: number;
    streamingAssistant: string | null;
    streamingReasoning: string | null;
    isStreaming: boolean;
    lastError: ChatErrorPayload | null;
    abortController: AbortController | null;
    /**
     * M9-DM: per-turn reasoning aggregation. Keyed by turn id (assistant
     * message id once known, or a transient stream id while in-flight). The
     * single-active `streamingReasoning` field above is kept for back-compat
     * with the current ChatPanel render path; this map is the forward-compat
     * shape for multi-turn / re-render scenarios introduced by tasks landing
     * after the M9 ReasoningPill polish.
     */
    reasoningStreams: Map<string, string>;

    appendUser: (content: string, parts?: MessagePart[]) => void;
    appendAssistantDelta: (delta: string) => void;
    appendReasoningDelta: (text: string) => void;
    /** Append a reasoning delta for a specific turn id. */
    appendReasoning: (turnId: string, delta: string) => void;
    /** Marker hook for end-of-reasoning; reserved for future totalTokens propagation. */
    finalizeReasoning: (turnId: string) => void;
    finalizeAssistant: () => void;
    /** Replace the entire history (used after loading from /sessions/:id/messages). */
    setMessages: (messages: ChatMessage[]) => void;
    reset: () => void;

    /**
     * Remove the message with the given id AND every message after it, then
     * clear all transient turn state (stream events, partial buffers, errors,
     * reasoning streams, and isStreaming).  _nextSeq is set to
     * (max surviving sequenceIndex) + 1 so subsequent appends stay monotonic
     * and do not collide with earlier sequence indices.
     *
     * @remarks Intended to be called when no turn is actively streaming. The
     * retry path guards `isStreaming` before calling this action. A caller that
     * invokes it mid-stream will clear the abort controller without aborting the
     * live request, leaving an orphaned in-flight fetch.
     */
    truncateTo: (messageId: string) => void;

    /** Mark the start of a stream and store the controller used to abort it. */
    beginStream: (controller: AbortController) => void;
    /** Mark the stream ended (success or error). Clears the controller. */
    endStream: () => void;
    /** Trigger an abort on the active controller, if any. */
    abort: () => void;
    setError: (err: ChatErrorPayload | null) => void;

    /** Record a tool-call start; placed inline in the merged stream. */
    addToolCallStartEvent: (id: string, toolName: string, args: unknown, round: number) => void;
    /** Settle an existing tool-call event with its result. */
    settleToolCallEvent: (id: string, result: unknown, isError: boolean) => void;
    /** Clear all stream events (called at stream start so prior turn cards are gone). */
    clearStreamEvents: () => void;
  };
}

export const createChatSlice: StateCreator<ChatSlice, [], [], ChatSlice> = (set, get) => ({
  chat: {
    messages: [],
    chatStreamEvents: [],
    _nextSeq: 0,
    streamingAssistant: null,
    streamingReasoning: null,
    isStreaming: false,
    lastError: null,
    abortController: null,
    reasoningStreams: new Map<string, string>(),

    appendUser: (content, parts) =>
      set((s) => {
        const seq = s.chat._nextSeq;
        const msg: ChatMessage = { id: newMessageId(), role: 'user', content, sequenceIndex: seq };
        if (parts && parts.length > 0) msg.parts = parts;
        return {
          chat: {
            ...s.chat,
            _nextSeq: seq + 1,
            messages: [...s.chat.messages, msg],
          },
        };
      }),

    setMessages: (messages) =>
      set((s) => {
        // Recompute _nextSeq so it is always greater than every loaded message's
        // sequenceIndex. Mirrors the same scan used in truncateTo.
        let nextSeq = 0;
        for (const m of messages) {
          if (m.sequenceIndex !== undefined && m.sequenceIndex >= nextSeq) {
            nextSeq = m.sequenceIndex + 1;
          }
        }
        return { chat: { ...s.chat, messages, _nextSeq: nextSeq } };
      }),

    appendAssistantDelta: (delta) => {
      if (delta.length === 0) return;
      set((s) => ({
        chat: {
          ...s.chat,
          streamingAssistant: (s.chat.streamingAssistant ?? '') + delta,
        },
      }));
    },

    appendReasoningDelta: (text) => {
      if (text.length === 0) return;
      set((s) => ({
        chat: {
          ...s.chat,
          streamingReasoning: (s.chat.streamingReasoning ?? '') + text,
        },
      }));
    },

    appendReasoning: (turnId, delta) => {
      if (delta.length === 0) return;
      set((s) => {
        const next = new Map(s.chat.reasoningStreams);
        next.set(turnId, (next.get(turnId) ?? '') + delta);
        return { chat: { ...s.chat, reasoningStreams: next } };
      });
    },

    finalizeReasoning: (_turnId) => {
      // Marker action for future totalTokens propagation from the
      // reasoning_text_end SSE event. Currently a no-op: the UI flips
      // ReasoningPill.isStreaming via the next text_delta arrival.
    },

    finalizeAssistant: () => {
      const current = get().chat.streamingAssistant;
      // null means no turn was ever started - nothing to finalize.
      if (current === null) return;
      // Empty string means a turn started but produced no text; write a placeholder
      // so the conversation log always has a visible assistant entry. Localized via
      // the i18next singleton (defaultValue keeps it stable if i18n isn't initialised,
      // e.g. in slice unit tests). [F4]
      const content =
        current.length === 0
          ? i18next.isInitialized
            ? i18next.t('chat:no_response')
            : '(no response)'
          : current;
      set((s) => {
        const seq = s.chat._nextSeq;
        return {
          chat: {
            ...s.chat,
            _nextSeq: seq + 1,
            messages: [
              ...s.chat.messages,
              { id: newMessageId(), role: 'assistant', content, sequenceIndex: seq },
            ],
            streamingAssistant: null,
            streamingReasoning: null,
          },
        };
      });
    },

    reset: () =>
      set((s) => ({
        chat: {
          ...s.chat,
          messages: [],
          chatStreamEvents: [],
          _nextSeq: 0,
          streamingAssistant: null,
          streamingReasoning: null,
          reasoningStreams: new Map<string, string>(),
          lastError: null,
        },
      })),

    truncateTo: (messageId) =>
      set((s) => {
        const cutIdx = s.chat.messages.findIndex((m) => m.id === messageId);
        // If the message is not found, leave state unchanged.
        if (cutIdx === -1) return s;
        const surviving = s.chat.messages.slice(0, cutIdx);
        // Compute the next sequence index so appends after the truncate are
        // monotonically ordered and never collide with surviving message indices.
        let nextSeq = 0;
        for (const m of surviving) {
          if (m.sequenceIndex !== undefined && m.sequenceIndex >= nextSeq) {
            nextSeq = m.sequenceIndex + 1;
          }
        }
        return {
          chat: {
            ...s.chat,
            messages: surviving,
            _nextSeq: nextSeq,
            chatStreamEvents: [],
            streamingAssistant: null,
            streamingReasoning: null,
            reasoningStreams: new Map<string, string>(),
            lastError: null,
            abortController: null,
            isStreaming: false,
          },
        };
      }),

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

    addToolCallStartEvent: (id, toolName, args, round) =>
      set((s) => {
        const seq = s.chat._nextSeq;
        const event: ChatStreamEvent = {
          id,
          toolName,
          sequenceIndex: seq,
          status: 'pending',
          args,
          result: null,
          isError: false,
          round,
        };
        return {
          chat: {
            ...s.chat,
            _nextSeq: seq + 1,
            chatStreamEvents: [...s.chat.chatStreamEvents, event],
          },
        };
      }),

    settleToolCallEvent: (id, result, isError) =>
      set((s) => ({
        chat: {
          ...s.chat,
          chatStreamEvents: s.chat.chatStreamEvents.map((e) =>
            e.id === id ? { ...e, result, isError, status: isError ? 'error' : 'success' } : e,
          ),
        },
      })),

    clearStreamEvents: () =>
      set((s) => ({
        chat: {
          ...s.chat,
          chatStreamEvents: [],
        },
      })),
  },
});
