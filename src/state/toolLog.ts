import type { StateCreator } from 'zustand';
import type { ImageSource } from '../api/contracts/agent';

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
  /** Data URL of an image produced by this tool call, if any. */
  imageDataUrl?: string;
  /** Routing kind of the attached image. */
  imageKind?: 'map' | 'chat';
  /** Provenance for the ephemeral generated or bundled image. */
  imageSource?: ImageSource;
  /** Data URL of a video clip produced by this tool call, if any. */
  videoDataUrl?: string;
}

export interface ToolLogSlice {
  toolLog: {
    entries: ToolLogEntry[];
    isOpen: boolean;
    addPending: (id: string, toolName: string, args: unknown, round: number) => void;
    settle: (id: string, result: unknown, isError: boolean, handledBy: string) => void;
    attachImage: (id: string, dataUrl: string, kind: 'map' | 'chat', source: ImageSource) => void;
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

    attachImage: (id, dataUrl, kind, source) =>
      set((s) => ({
        toolLog: {
          ...s.toolLog,
          entries: s.toolLog.entries.map((entry) =>
            entry.id === id
              ? { ...entry, imageDataUrl: dataUrl, imageKind: kind, imageSource: source }
              : entry,
          ),
        },
      })),

    clear: () => set((s) => ({ toolLog: { ...s.toolLog, entries: [] } })),
    open: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: true } })),
    close: () => set((s) => ({ toolLog: { ...s.toolLog, isOpen: false } })),
  },
});
