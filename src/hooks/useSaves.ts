/**
 * useSaves (M5 P2.13)
 *
 * Wires the Zustand `saves` slice to the backend API client. Returns
 * stable callbacks so consumers (the Saves modal, the Ctrl+S hotkey)
 * can refresh / mutate without re-binding effects.
 */

import { useCallback, useMemo } from 'react';
import {
  createSave as apiCreateSave,
  type CreateSaveRequest,
  deleteSaveById,
  fetchSaveById,
  fetchSessionMessages,
  fetchSessionSaves,
  quickSaveSession,
  type SaveSummary,
  type SessionMessageWire,
  updateSaveById,
} from '../api/saves';
import type { ChatMessage, ChatRole, MessagePart } from '../state/chat';
import type { PcData } from '../state/pc';
import { useStore } from '../state/useStore';

const REHYDRATE_LIMIT = 20;

// Only "user", "assistant", and "system" messages are rendered by the V1 chat UI.
// "assistant_with_tool_calls" and "tool_result" variants are filtered out during
// V1 rehydration - see the comment in rehydrateFromSave.
const RENDERABLE_ROLES = new Set<string>(['user', 'assistant', 'system']);

export type RehydrateResult = { ok: true } | { ok: false; error: string };

export interface UseSavesResult {
  saves: SaveSummary[];
  selectedSaveId: string | null;
  isOpen: boolean;
  lastQuickSaveAt: string | null;
  refresh: () => Promise<void>;
  quickSave: () => Promise<{ id: string } | null>;
  manualSave: (body: CreateSaveRequest) => Promise<{ id: string } | null>;
  overwriteSave: (saveId: string, body: CreateSaveRequest) => Promise<boolean>;
  rehydrateFromSave: (saveId: string) => Promise<RehydrateResult>;
  deleteSave: (saveId: string) => Promise<void>;
  open: () => void;
  close: () => void;
  selectSave: (id: string | null) => void;
}

export function useSaves(): UseSavesResult {
  const saves = useStore((s) => s.saves.list);
  const selectedSaveId = useStore((s) => s.saves.selectedSaveId);
  const isOpen = useStore((s) => s.saves.isOpen);
  const lastQuickSaveAt = useStore((s) => s.saves.lastQuickSaveAt);

  const setList = useStore((s) => s.saves.setList);
  const selectSave = useStore((s) => s.saves.selectSave);
  const open = useStore((s) => s.saves.open);
  const close = useStore((s) => s.saves.close);
  const setLastQuickSaveAt = useStore((s) => s.saves.setLastQuickSaveAt);
  const setLastSaveError = useStore((s) => s.saves.setLastSaveError);
  const ensureSession = useStore((s) => s.session.ensureSession);
  const setActiveSession = useStore((s) => s.session.setActiveSession);
  const setMessages = useStore((s) => s.chat.setMessages);
  const replaceFromDraft = useStore((s) => s.pc.replaceFromDraft);

  const refresh = useCallback(async () => {
    const { sessionId } = ensureSession();
    const list = await fetchSessionSaves(sessionId);
    setList(list);
  }, [ensureSession, setList]);

  const quickSave = useCallback(async () => {
    const { sessionId } = ensureSession();
    try {
      const result = await quickSaveSession(sessionId);
      setLastQuickSaveAt(new Date().toISOString());
      // Best-effort refresh; quick save is fire-and-forget so swallow refresh errors.
      try {
        const list = await fetchSessionSaves(sessionId);
        setList(list);
      } catch {
        // ignore - the toast is the user-visible signal
      }
      setLastSaveError(null);
      return result;
    } catch (e) {
      console.error('quickSave failed', e);
      setLastSaveError(e instanceof Error ? e.message : String(e));
      return null;
    }
  }, [ensureSession, setLastQuickSaveAt, setList, setLastSaveError]);

  const manualSave = useCallback(
    async (body: CreateSaveRequest) => {
      const { sessionId } = ensureSession();
      try {
        const result = await apiCreateSave(sessionId, body);
        const list = await fetchSessionSaves(sessionId);
        setList(list);
        setLastSaveError(null);
        return result;
      } catch (e) {
        console.error('manualSave failed', e);
        setLastSaveError(e instanceof Error ? e.message : String(e));
        return null;
      }
    },
    [ensureSession, setList, setLastSaveError],
  );

  const overwriteSave = useCallback(
    async (saveId: string, body: CreateSaveRequest) => {
      const { sessionId } = ensureSession();
      try {
        await updateSaveById(saveId, body);
        const list = await fetchSessionSaves(sessionId);
        setList(list);
        setLastSaveError(null);
        return true;
      } catch (e) {
        console.error('overwriteSave failed', e);
        setLastSaveError(e instanceof Error ? e.message : String(e));
        return false;
      }
    },
    [ensureSession, setList, setLastSaveError],
  );

  /**
   * Rehydrate the active session from a save row.
   *
   * Steps:
   *   (a) Fetch the full save row.
   *   (b) Fetch the last 20 messages for that session.
   *   (c) Build the ChatMessage array from the wire messages.
   *   (d) Perform all store mutations atomically (only after all fetches succeed):
   *         setActiveSession(...), setMessages(...), replaceFromDraft(...).
   *   (e) If the save's game_state carries a pc_snapshot, apply it.
   *
   * NOTE: Combat rehydration is deferred to v2 - the V1 save envelope
   * carries no combat snapshot data, so there is nothing to restore here.
   *
   * IMPORTANT: All fallible fetches are performed BEFORE any store mutation.
   * This ensures that a fetch failure writes nothing to the store and leaves
   * the app state consistent without requiring a rollback.
   */
  const rehydrateFromSave = useCallback(
    async (saveId: string): Promise<RehydrateResult> => {
      try {
        // (a) Fetch the full save row (provides session_id + game_state).
        const row = await fetchSaveById(saveId);

        // (b) Fetch the last 20 messages for this session.
        // fetchSessionMessages also slices client-side (the backend currently ignores
        // ?limit=), but we apply the limit here too so the contract holds even if the
        // fetch layer is swapped or mocked.
        const allWireMessages = await fetchSessionMessages(row.session_id, {
          limit: REHYDRATE_LIMIT,
        });
        const wireMessages =
          allWireMessages.length > REHYDRATE_LIMIT
            ? allWireMessages.slice(-REHYDRATE_LIMIT)
            : allWireMessages;

        // (c) Filter to renderable roles only, then assign positional frontend ids
        // (stable within this single rehydration pass; the store does a full replace
        // each time so positional ids are sufficient - they are not content-stable
        // across separate loads if the filtered set changes).
        // The backend ChatMessage enum also emits "assistant_with_tool_calls" and
        // "tool_result" variants which the V1 chat UI (ChatRole in state/chat.ts)
        // does not render. These are deliberately skipped during V1 rehydration;
        // full tool-call rehydration is deferred to a future milestone.
        const messages: ChatMessage[] = wireMessages
          .filter((wm) => RENDERABLE_ROLES.has(wm.role))
          .map((wm: SessionMessageWire, idx: number) => {
            const content =
              wm.content ??
              wm.parts
                ?.filter((p) => p.type === 'text')
                .map((p) => p.text ?? '')
                .join('') ??
              '';
            // After the RENDERABLE_ROLES filter above, wm.role is guaranteed to be
            // a valid ChatRole. The cast is necessary because TypeScript cannot
            // narrow the type through a Set.has() guard on the filtered array.
            const msg: ChatMessage = {
              id: `rehydrated-${saveId}-${idx}`,
              role: wm.role as ChatRole,
              content,
            };
            if (wm.parts && wm.parts.length > 0) {
              const parts: MessagePart[] = wm.parts
                .map((p): MessagePart | null => {
                  if (p.type === 'text' && p.text !== undefined) {
                    return { type: 'text', text: p.text };
                  }
                  if (p.type === 'image' && p.mime !== undefined && p.data_b64 !== undefined) {
                    const imgPart: MessagePart = {
                      type: 'image',
                      mime: p.mime,
                      data_b64: p.data_b64,
                    };
                    if (p.name !== undefined && p.name !== null) {
                      return { ...imgPart, name: p.name };
                    }
                    return imgPart;
                  }
                  return null;
                })
                .filter((p): p is MessagePart => p !== null);
              if (parts.length > 0) msg.parts = parts;
            }
            return msg;
          });

        // (d) All fetches succeeded - perform store mutations atomically.
        // V1 assumption: the loaded session belongs to the currently-active campaign
        // because the V1 save schema does not store a campaign_id. Cross-campaign
        // save browsing would require the schema to carry a campaign_id field.
        const { campaignId } = ensureSession();
        setActiveSession(campaignId, row.session_id);
        setMessages(messages);

        // (e) If the V1 save envelope carries a pc_snapshot, apply it.
        // The V1 envelope does not include pc_snapshot, so this guard normally
        // never fires. It is written defensively for forward-compatibility
        // with future save envelope versions.
        const gameState = row.game_state as Record<string, unknown> | null | undefined;
        if (gameState && typeof gameState === 'object' && 'pc_snapshot' in gameState) {
          const snapshot = gameState.pc_snapshot;
          if (snapshot && typeof snapshot === 'object') {
            replaceFromDraft(snapshot as Partial<PcData>);
          }
        }

        return { ok: true };
      } catch (e) {
        const message = e instanceof Error ? e.message : 'Load failed';
        console.error('rehydrateFromSave failed', e);
        return { ok: false, error: message };
      }
    },
    [ensureSession, setActiveSession, setMessages, replaceFromDraft],
  );

  const deleteSave = useCallback(
    async (saveId: string) => {
      try {
        await deleteSaveById(saveId);
        await refresh();
        setLastSaveError(null);
      } catch (e) {
        console.error('deleteSave failed', e);
        setLastSaveError(e instanceof Error ? e.message : String(e));
      }
    },
    [refresh, setLastSaveError],
  );

  return useMemo(
    () => ({
      saves,
      selectedSaveId,
      isOpen,
      lastQuickSaveAt,
      refresh,
      quickSave,
      manualSave,
      overwriteSave,
      rehydrateFromSave,
      deleteSave,
      open,
      close,
      selectSave,
    }),
    [
      saves,
      selectedSaveId,
      isOpen,
      lastQuickSaveAt,
      refresh,
      quickSave,
      manualSave,
      overwriteSave,
      rehydrateFromSave,
      deleteSave,
      open,
      close,
      selectSave,
    ],
  );
}
