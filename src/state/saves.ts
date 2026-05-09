/**
 * Saves UI state slice (M5 P2.13).
 *
 * Pure UI state: open/close + selected-save id + the cached list of
 * saves shown in the tome modal + the last quick-save timestamp used by
 * the toast hint. Persistence is intentionally NOT enabled - the canonical
 * source of truth is the backend (`/sessions/{id}/saves`).
 */

import type { StateCreator } from 'zustand';
import type { SaveSummary } from '../api/saves';

export interface SavesData {
  isOpen: boolean;
  /** id of the save currently shown in the right page; null when none selected */
  selectedSaveId: string | null;
  /** Cached list - refreshed via the `useSaves` hook on open + after mutations */
  list: SaveSummary[];
  /** ISO timestamp of the last successful quick save, used by the Saved-now toast */
  lastQuickSaveAt: string | null;
}

export interface SavesActions {
  open: () => void;
  close: () => void;
  setList: (list: SaveSummary[]) => void;
  selectSave: (id: string | null) => void;
  setLastQuickSaveAt: (iso: string | null) => void;
}

export interface SavesSlice {
  saves: SavesData & SavesActions;
}

export const createSavesSlice: StateCreator<SavesSlice, [], [], SavesSlice> = (set) => ({
  saves: {
    isOpen: false,
    selectedSaveId: null,
    list: [],
    lastQuickSaveAt: null,
    open: () => set((s) => ({ saves: { ...s.saves, isOpen: true } })),
    close: () => set((s) => ({ saves: { ...s.saves, isOpen: false } })),
    setList: (list) =>
      set((s) => {
        // Auto-select the newest save when the panel currently has no selection
        // (or its previously-selected id no longer exists in the list).
        const stillExists =
          s.saves.selectedSaveId !== null && list.some((x) => x.id === s.saves.selectedSaveId);
        const selectedSaveId = stillExists ? s.saves.selectedSaveId : (list[0]?.id ?? null);
        return { saves: { ...s.saves, list, selectedSaveId } };
      }),
    selectSave: (id) => set((s) => ({ saves: { ...s.saves, selectedSaveId: id } })),
    setLastQuickSaveAt: (iso) => set((s) => ({ saves: { ...s.saves, lastQuickSaveAt: iso } })),
  },
});
