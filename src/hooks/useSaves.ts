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
  restoreSave,
  type SaveSummary,
  type SessionMessageWire,
  updateSaveById,
} from '../api/saves';
import type { ChatMessage, ChatRole, MessagePart } from '../state/chat';
import type { SnapshotCombat } from '../state/combat';
import type { PcData } from '../state/pc';
import type { CurrentScene } from '../state/session';
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
  const setCurrentScene = useStore((s) => s.session.setCurrentScene);
  const setMessages = useStore((s) => s.chat.setMessages);
  const replaceFromDraft = useStore((s) => s.pc.replaceFromDraft);
  const hydrateCombat = useStore((s) => s.combat.hydrate);
  const endCombat = useStore((s) => s.combat.endCombat);

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
   *   (a) Fetch the full save row (for session_id).
   *   (b) Call POST /saves/{id}/restore to restore combat DB state server-side
   *       and receive the schema-v2 game_state.
   *   (c) Fetch the last 20 messages for that session.
   *   (d) Build the ChatMessage array from the wire messages.
   *   (e) Perform all store mutations (only after all fetches succeed):
   *         setActiveSession, setMessages, replaceFromDraft,
   *         hydrateCombat (if combat present) or endCombat (if not),
   *         setCurrentScene (if scene present).
   *   (f) If the save's game_state carries a pc_snapshot, apply it.
   *
   * NOTE: Chat history rehydration is partial (last 20 renderable messages).
   * Full tool-call rehydration and branching are deferred to a future milestone.
   *
   * IMPORTANT: All fallible fetches are performed BEFORE any store mutation.
   * This ensures that a fetch failure writes nothing to the store and leaves
   * the app state consistent without requiring a rollback.
   */
  const rehydrateFromSave = useCallback(
    async (saveId: string): Promise<RehydrateResult> => {
      try {
        // (a) Fetch the full save row (provides session_id + raw game_state for pc_snapshot).
        const row = await fetchSaveById(saveId);

        // (b) Call restore endpoint to rehydrate combat DB state server-side.
        // Returns the schema-v2 game_state with combat + scene.
        // We call this with the session_id from the save row.
        const { campaignId } = ensureSession();
        const restored = await restoreSave(saveId, row.session_id);
        const restoredGs = restored.game_state as Record<string, unknown> | null | undefined;

        // (c) Fetch the last 20 messages for this session.
        const allWireMessages = await fetchSessionMessages(row.session_id, {
          limit: REHYDRATE_LIMIT,
        });
        const wireMessages =
          allWireMessages.length > REHYDRATE_LIMIT
            ? allWireMessages.slice(-REHYDRATE_LIMIT)
            : allWireMessages;

        // (d) Filter to renderable roles only, then assign positional frontend ids.
        // "assistant_with_tool_calls" and "tool_result" variants are filtered out
        // during V1 rehydration; full tool-call rehydration is deferred.
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

        // (e) All fetches succeeded - perform store mutations.
        setActiveSession(campaignId, row.session_id);
        setMessages(messages);

        // Rehydrate combat slice from schema-v2 game_state.
        const isV2 =
          restoredGs != null &&
          typeof restoredGs === 'object' &&
          'schema_version' in restoredGs &&
          restoredGs.schema_version === 2;
        const savedCombat = isV2 ? (restoredGs?.combat as SnapshotCombat | null | undefined) : null;
        if (savedCombat?.active) {
          hydrateCombat(savedCombat);
        } else {
          endCombat();
        }

        // Rehydrate scene slice.
        type SavedScene = { title: string; subtitle?: string | null; mode: string };
        const savedScene = isV2 ? (restoredGs?.scene as SavedScene | null | undefined) : null;
        if (savedScene?.title) {
          const scene: CurrentScene = { name: savedScene.title, stepCounter: 0 };
          setCurrentScene(scene);
        } else {
          setCurrentScene(null);
        }

        // (f) pc_snapshot (forward-compat; not present in v1 or v2 yet).
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
    [
      ensureSession,
      setActiveSession,
      setCurrentScene,
      setMessages,
      replaceFromDraft,
      hydrateCombat,
      endCombat,
    ],
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
