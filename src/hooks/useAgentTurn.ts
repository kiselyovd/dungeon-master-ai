import { useCallback } from 'react';
import { streamAgentTurn } from '../api/agent';
import { ChatError } from '../api/errors';
import { DISPOSITIONS, type Disposition } from '../state/npc';
import { useStore } from '../state/useStore';

const HARDCODED_CAMPAIGN_ID = '00000000-0000-0000-0000-000000000001';
const HARDCODED_SESSION_ID = '00000000-0000-0000-0000-000000000002';

/**
 * Agent turn orchestrator hook. Replaces useChat.send for the M3 agent endpoint.
 * Dispatches text deltas, tool-call events, journal entries, and NPC updates
 * to the appropriate Zustand slices. Campaign/session IDs are hardcoded for
 * M3; M5 will plumb them through a SessionSlice.
 */
export function useAgentTurn() {
  const appendUser = useStore((s) => s.chat.appendUser);
  const appendDelta = useStore((s) => s.chat.appendAssistantDelta);
  const finalize = useStore((s) => s.chat.finalizeAssistant);
  const beginStream = useStore((s) => s.chat.beginStream);
  const endStream = useStore((s) => s.chat.endStream);
  const setError = useStore((s) => s.chat.setError);
  const abort = useStore((s) => s.chat.abort);
  const addPending = useStore((s) => s.toolLog.addPending);
  const settle = useStore((s) => s.toolLog.settle);
  const appendJournalEntry = useStore((s) => s.journal.appendEntry);
  const upsertNpc = useStore((s) => s.npcs.upsertNpc);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim()) return;
      if (useStore.getState().chat.isStreaming) return;

      appendUser(text);
      const controller = new AbortController();
      beginStream(controller);

      const history = useStore.getState().chat.messages;

      try {
        await streamAgentTurn({
          campaignId: HARDCODED_CAMPAIGN_ID,
          sessionId: HARDCODED_SESSION_ID,
          playerMessage: text,
          history,
          signal: controller.signal,

          onTextDelta: appendDelta,

          onToolCallStart: (id, toolName, round) => {
            addPending(id, toolName, {}, round);
          },

          onToolCallResult: (id, toolName, args, result, isError, _round) => {
            settle(id, result, isError);

            if (toolName === 'journal_append' && !isError && result && typeof result === 'object') {
              const r = result as Record<string, unknown>;
              if (r.entry_id) {
                const a = (args ?? {}) as Record<string, unknown>;
                appendJournalEntry({
                  id: String(r.entry_id),
                  campaign_id: HARDCODED_CAMPAIGN_ID,
                  chapter: typeof a.chapter === 'string' ? a.chapter : null,
                  entry_html: typeof a.entry_html === 'string' ? a.entry_html : '',
                  created_at: new Date().toISOString(),
                });
              }
            }

            if (toolName === 'remember_npc' && !isError && args && typeof args === 'object') {
              const a = args as Record<string, unknown>;
              const name = String(a.name ?? '');
              if (name) {
                const disposition: Disposition =
                  typeof a.disposition === 'string' &&
                  (DISPOSITIONS as readonly string[]).includes(a.disposition)
                    ? (a.disposition as Disposition)
                    : 'unknown';
                upsertNpc({
                  id: name,
                  campaign_id: HARDCODED_CAMPAIGN_ID,
                  name,
                  role: typeof a.role === 'string' ? a.role : '',
                  disposition,
                  trust: 0,
                  facts: [
                    {
                      text: typeof a.fact === 'string' ? a.fact : '',
                      created_at: new Date().toISOString(),
                    },
                  ],
                  updated_at: new Date().toISOString(),
                });
              }
            }
          },

          onAgentDone: () => {
            // Agent loop complete; chat slice's finalize() runs in finally.
          },
        });
      } catch (e) {
        setError(ChatError.from(e).toPayload());
      } finally {
        finalize();
        endStream();
      }
    },
    [
      appendUser,
      appendDelta,
      finalize,
      beginStream,
      endStream,
      setError,
      addPending,
      settle,
      appendJournalEntry,
      upsertNpc,
    ],
  );

  return { send, cancel: abort };
}
