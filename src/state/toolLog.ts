import type { StateCreator } from 'zustand';

export interface ToolLogEntry {
  id: string;
  toolName: string;
  args: unknown;
  result: unknown | null;
  isError: boolean;
  round: number;
  timestamp: string;
}

export interface ToolLogSlice {
  toolLog: {
    entries: ToolLogEntry[];
    isOpen: boolean;
    addPending: (id: string, toolName: string, args: unknown, round: number) => void;
    settle: (id: string, result: unknown, isError: boolean) => void;
    clear: () => void;
    open: () => void;
    close: () => void;
  };
}

export const createToolLogSlice: StateCreator<ToolLogSlice, [], [], ToolLogSlice> = (set) => ({
  toolLog: {
    entries: [],
    isOpen: false,

    addPending: (id, toolName, args, round) =>
      set((s) => ({
        toolLog: {
          ...s.toolLog,
          entries: [
            ...s.toolLog.entries,
            {
              id,
              toolName,
              args,
              result: null,
              isError: false,
              round,
              timestamp: new Date().toISOString(),
            },
          ],
        },
      })),

    settle: (id, result, isError) =>
      set((s) => ({
        toolLog: {
          ...s.toolLog,
          entries: s.toolLog.entries.map((e) => (e.id === id ? { ...e, result, isError } : e)),
        },
      })),

    clear: () => set((s) => ({ toolLog: { ...s.toolLog, entries: [] } })),
    open: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: true } })),
    close: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: false } })),
  },
});
