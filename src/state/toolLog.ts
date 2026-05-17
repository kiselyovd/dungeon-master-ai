import type { StateCreator } from 'zustand';

export interface ToolLogEntry {
  id: string;
  toolName: string;
  args: unknown;
  result: unknown | null;
  isError: boolean;
  round: number;
  timestamp: string;
  /// M7.5-DM: classifier from the backend identifying which subsystem ran
  /// this tool ("engine", "image-provider", ...). Surfaced as a pill in the
  /// Tool Inspector so users can distinguish engine deterministic execution
  /// from provider delegation. Defaults to "engine" before settle().
  handledBy: string;
}

export interface ToolLogSlice {
  toolLog: {
    entries: ToolLogEntry[];
    isOpen: boolean;
    addPending: (id: string, toolName: string, args: unknown, round: number) => void;
    settle: (id: string, result: unknown, isError: boolean, handledBy: string) => void;
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
              handledBy: 'engine',
            },
          ],
        },
      })),

    settle: (id, result, isError, handledBy) =>
      set((s) => ({
        toolLog: {
          ...s.toolLog,
          entries: s.toolLog.entries.map((e) =>
            e.id === id ? { ...e, result, isError, handledBy } : e,
          ),
        },
      })),

    clear: () => set((s) => ({ toolLog: { ...s.toolLog, entries: [] } })),
    open: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: true } })),
    close: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: false } })),
  },
});
