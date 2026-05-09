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
  fetchSessionSaves,
  quickSaveSession,
  type SaveRow,
  type SaveSummary,
} from '../api/saves';
import { useStore } from '../state/useStore';

export interface UseSavesResult {
  saves: SaveSummary[];
  selectedSaveId: string | null;
  isOpen: boolean;
  lastQuickSaveAt: string | null;
  refresh: () => Promise<void>;
  quickSave: () => Promise<{ id: string } | null>;
  manualSave: (body: CreateSaveRequest) => Promise<{ id: string } | null>;
  loadSave: (saveId: string) => Promise<SaveRow | null>;
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
  const ensureSession = useStore((s) => s.session.ensureSession);

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
      return result;
    } catch (e) {
      // Surface in dev-tools but never break the host (no toast yet).
      console.error('quickSave failed', e);
      return null;
    }
  }, [ensureSession, setLastQuickSaveAt, setList]);

  const manualSave = useCallback(
    async (body: CreateSaveRequest) => {
      const { sessionId } = ensureSession();
      try {
        const result = await apiCreateSave(sessionId, body);
        const list = await fetchSessionSaves(sessionId);
        setList(list);
        return result;
      } catch (e) {
        console.error('manualSave failed', e);
        return null;
      }
    },
    [ensureSession, setList],
  );

  const loadSave = useCallback(async (saveId: string) => {
    try {
      return await fetchSaveById(saveId);
    } catch (e) {
      console.error('loadSave failed', e);
      return null;
    }
  }, []);

  const deleteSave = useCallback(
    async (saveId: string) => {
      try {
        await deleteSaveById(saveId);
        await refresh();
      } catch (e) {
        console.error('deleteSave failed', e);
      }
    },
    [refresh],
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
      loadSave,
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
      loadSave,
      deleteSave,
      open,
      close,
      selectSave,
    ],
  );
}
