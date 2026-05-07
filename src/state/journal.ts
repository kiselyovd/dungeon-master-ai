import type { StateCreator } from 'zustand';

export interface JournalEntry {
  id: string;
  campaign_id: string;
  chapter: string | null;
  entry_html: string;
  created_at: string;
}

export interface JournalSlice {
  journal: {
    entries: JournalEntry[];
    isOpen: boolean;
    appendEntry: (entry: JournalEntry) => void;
    setEntries: (entries: JournalEntry[]) => void;
    open: () => void;
    close: () => void;
  };
}

export const createJournalSlice: StateCreator<JournalSlice, [], [], JournalSlice> = (set) => ({
  journal: {
    entries: [],
    isOpen: false,
    appendEntry: (entry) =>
      set((s) => ({ journal: { ...s.journal, entries: [...s.journal.entries, entry] } })),
    setEntries: (entries) => set((s) => ({ journal: { ...s.journal, entries } })),
    open: () => set((s) => ({ journal: { ...s.journal, isOpen: true } })),
    close: () => set((s) => ({ journal: { ...s.journal, isOpen: false } })),
  },
});
