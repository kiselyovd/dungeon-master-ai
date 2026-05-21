import { useCallback } from 'react';
import { streamAgentTurn } from '../api/agent';
import { ChatError } from '../api/errors';
import { DISPOSITIONS, type Disposition } from '../state/npc';
import { useStore } from '../state/useStore';
import { combatToolHandlers } from './useCombatToolHandlers';

/**
 * Agent turn orchestrator hook. Replaces useChat.send for the M3 agent endpoint.
 * Dispatches text deltas, tool-call events, journal entries, and NPC updates
 * to the appropriate Zustand slices. Campaign/session IDs come from the
 * persistent SessionSlice (M5); each launch lazily mints a UUID pair via
 * `ensureSession()` on first mount.
 */
export function useAgentTurn() {
  const appendUser = useStore((s) => s.chat.appendUser);
  const appendDelta = useStore((s) => s.chat.appendAssistantDelta);
  const finalize = useStore((s) => s.chat.finalizeAssistant);
  const beginStream = useStore((s) => s.chat.beginStream);
  const endStream = useStore((s) => s.chat.endStream);
  const setError = useStore((s) => s.chat.setError);
  const abort = useStore((s) => s.chat.abort);
  const addToolCallStartEvent = useStore((s) => s.chat.addToolCallStartEvent);
  const settleToolCallEvent = useStore((s) => s.chat.settleToolCallEvent);
  const clearStreamEvents = useStore((s) => s.chat.clearStreamEvents);
  const addPending = useStore((s) => s.toolLog.addPending);
  const settle = useStore((s) => s.toolLog.settle);
  const appendJournalEntry = useStore((s) => s.journal.appendEntry);
  const upsertNpc = useStore((s) => s.npcs.upsertNpc);
  const ensureSession = useStore((s) => s.session.ensureSession);

  const send = useCallback(
    async (text: string) => {
      if (!text.trim()) return;
      if (useStore.getState().chat.isStreaming) return;

      const { campaignId, sessionId } = ensureSession();

      clearStreamEvents();
      appendUser(text);
      const controller = new AbortController();
      beginStream(controller);

      const history = useStore.getState().chat.messages;

      try {
        await streamAgentTurn({
          campaignId,
          sessionId,
          playerMessage: text,
          history,
          signal: controller.signal,

          onTextDelta: appendDelta,

          onReasoningDelta: (text) => {
            useStore.getState().chat.appendReasoningDelta(text);
          },

          onToolCallStart: (id, toolName, round) => {
            addPending(id, toolName, {}, round);
            addToolCallStartEvent(id, toolName, {}, round);
          },

          onToolCallResult: (id, toolName, args, result, isError, _round, handledBy) => {
            settle(id, result, isError, handledBy);
            settleToolCallEvent(id, result, isError);

            if (toolName === 'journal_append' && !isError && result && typeof result === 'object') {
              const r = result as Record<string, unknown>;
              if (r.entry_id) {
                const a = (args ?? {}) as Record<string, unknown>;
                appendJournalEntry({
                  id: String(r.entry_id),
                  campaign_id: campaignId,
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
                  campaign_id: campaignId,
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

            if (toolName === 'set_scene' && !isError && args && typeof args === 'object') {
              const a = args as Record<string, unknown>;
              const title = typeof a.title === 'string' ? a.title : '';
              if (title) {
                useStore.getState().session.setCurrentScene({ name: title, stepCounter: 0 });
              }
            }

            if (!isError && combatToolHandlers[toolName] !== undefined) {
              try {
                combatToolHandlers[toolName]?.(
                  (args ?? {}) as Record<string, unknown>,
                  (result ?? {}) as Record<string, unknown>,
                  useStore,
                );
              } catch (handlerErr) {
                console.error(`[combat handler "${toolName}"] threw:`, handlerErr);
              }
            }
          },

          onAgentDone: (_totalRounds) => {
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
      addToolCallStartEvent,
      settleToolCallEvent,
      clearStreamEvents,
      addPending,
      settle,
      appendJournalEntry,
      upsertNpc,
      ensureSession,
    ],
  );

  return { send, cancel: abort };
}
