import { useCallback } from 'react';
import { streamAgentTurn } from '../api/agent';
import { ChatError } from '../api/errors';
import type { MessagePart } from '../state/chat';
import type { CombatToken } from '../state/combat';
import { DISPOSITIONS, type Disposition } from '../state/npc';
import { type AppState, useStore } from '../state/useStore';
import { combatToolHandlers } from './useCombatToolHandlers';

/**
 * Build a compact, model-readable snapshot of the live VTT board so the DM
 * narrates from the actual state - whose turn it is, who is bloodied, and
 * where each combatant stands after the player drags tokens. Returns
 * `undefined` outside combat (no battlefield block is injected then).
 */
function buildBoardSnapshot(state: AppState): string | undefined {
  const combat = state.combat;
  if (!combat.active || combat.tokens.length === 0) return undefined;

  const byId = new Map<string, CombatToken>(combat.tokens.map((t) => [t.id, t]));
  const orderNames = combat.initiativeOrder
    .map((id) => byId.get(id))
    .filter((t): t is CombatToken => t !== undefined)
    .map((t) => (t.isActive ? `${t.name} (current turn)` : t.name));

  const lines = combat.tokens.map((t) => {
    const status = t.hp <= 0 ? ' - DOWN' : '';
    const conditions = t.conditions.length > 0 ? `, conditions: ${t.conditions.join(', ')}` : '';
    return `- ${t.name}: HP ${t.hp}/${t.maxHp}, AC ${t.ac}, grid (${t.x},${t.y})${conditions}${status}`;
  });

  const scene = state.session.currentScene?.name;
  const header = [
    scene ? `Scene: ${scene}.` : null,
    `Round ${combat.round}.`,
    orderNames.length > 0 ? `Initiative order: ${orderNames.join(' -> ')}.` : null,
  ]
    .filter((s): s is string => s !== null)
    .join(' ');

  return `${header}\nGrid squares are 5 ft. Combatants:\n${lines.join('\n')}`;
}

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
  const attachStreamEventVideo = useStore((s) => s.chat.attachStreamEventVideo);
  const addPending = useStore((s) => s.toolLog.addPending);
  const settle = useStore((s) => s.toolLog.settle);
  const appendJournalEntry = useStore((s) => s.journal.appendEntry);
  const upsertNpc = useStore((s) => s.npcs.upsertNpc);
  const ensureSession = useStore((s) => s.session.ensureSession);

  const send = useCallback(
    async (text: string, images?: MessagePart[]) => {
      if (!text.trim()) return;
      if (useStore.getState().chat.isStreaming) return;

      const { campaignId, sessionId } = ensureSession();

      clearStreamEvents();
      // Snapshot history BEFORE appending the current turn: the backend
      // orchestrator appends the current user message itself (from
      // player_message + the dedicated `images` field), so including it here
      // too would duplicate the turn in the LLM context. The local append is
      // only for rendering. [F2 / review]
      const history = useStore.getState().chat.messages;
      appendUser(text);
      const controller = new AbortController();
      beginStream(controller);

      // Snapshot the live board (positions/HP/turn) so the DM narrates from it.
      const board = buildBoardSnapshot(useStore.getState());

      try {
        await streamAgentTurn({
          campaignId,
          sessionId,
          playerMessage: text,
          history,
          ...(images && images.length > 0 ? { images } : {}),
          ...(board ? { board } : {}),
          signal: controller.signal,

          onTextDelta: appendDelta,

          onReasoningDelta: (text) => {
            useStore.getState().chat.appendReasoningDelta(text);
          },

          onImageGenerated: (dataUrl, toolCallId, kind) => {
            // Route by kind: a map paints the VTT board (left); an illustration
            // renders inline in its tool-call card (right). Both attach to the
            // card so the user sees the result in place of raw JSON.
            if (toolCallId) {
              useStore.getState().chat.attachStreamEventImage(toolCallId, dataUrl, kind);
            }
            if (kind === 'map') {
              useStore.getState().session.setMapImage(dataUrl);
            }
          },

          onVideoGenerated: (dataUrl, toolCallId) => {
            // Videos always render inline in the tool-call card (never the VTT board).
            if (toolCallId) {
              useStore.getState().chat.attachStreamEventVideo(toolCallId, dataUrl);
            }
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
      attachStreamEventVideo,
      addPending,
      settle,
      appendJournalEntry,
      upsertNpc,
      ensureSession,
    ],
  );

  return { send, cancel: abort };
}
