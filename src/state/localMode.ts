import type { StateCreator } from 'zustand';
import type { AppState } from './useStore';

export type ModelId = 'qwen3_5_0_8b' | 'qwen3_5_2b' | 'qwen3_5_4b' | 'qwen3_5_9b' | 'sdxl_turbo';
export type VramStrategy = 'auto-swap' | 'keep-both-loaded' | 'disable-image-gen';

export type RuntimeState =
  | { state: 'off' }
  | { state: 'starting' }
  | { state: 'ready'; port: number }
  | { state: 'failed'; reason: string };

export type DownloadState =
  | { state: 'idle' }
  | { state: 'downloading'; bytesDone: number; totalBytes: number | null }
  | { state: 'completed'; bytesTotal: number }
  | { state: 'failed'; reason: string };

export const ALL_MODEL_IDS: ModelId[] = [
  'qwen3_5_0_8b',
  'qwen3_5_2b',
  'qwen3_5_4b',
  'qwen3_5_9b',
  'sdxl_turbo',
];

export interface LocalModeData {
  enabled: boolean;
  selectedLlm: ModelId;
  vramStrategy: VramStrategy;
  runtime: { llm: RuntimeState; image: RuntimeState };
  downloads: Record<ModelId, DownloadState>;
}

export interface LocalModeSlice {
  localMode: LocalModeData & {
    setEnabled: (v: boolean) => void;
    selectModel: (id: ModelId) => void;
    setVramStrategy: (s: VramStrategy) => void;
    setRuntimeStatus: (snap: { llm: RuntimeState; image: RuntimeState }) => void;
    setDownloadState: (id: ModelId, s: DownloadState) => void;
  };
}

const idleDownloads = (): Record<ModelId, DownloadState> =>
  Object.fromEntries(ALL_MODEL_IDS.map((id) => [id, { state: 'idle' as const }])) as Record<
    ModelId,
    DownloadState
  >;

export const createLocalModeSlice: StateCreator<AppState, [], [], LocalModeSlice> = (set) => ({
  localMode: {
    enabled: false,
    selectedLlm: 'qwen3_5_4b',
    vramStrategy: 'auto-swap',
    runtime: { llm: { state: 'off' }, image: { state: 'off' } },
    downloads: idleDownloads(),
    setEnabled: (v) => set((s) => ({ localMode: { ...s.localMode, enabled: v } })),
    selectModel: (id) => set((s) => ({ localMode: { ...s.localMode, selectedLlm: id } })),
    setVramStrategy: (vs) => set((s) => ({ localMode: { ...s.localMode, vramStrategy: vs } })),
    setRuntimeStatus: (snap) => set((s) => ({ localMode: { ...s.localMode, runtime: snap } })),
    setDownloadState: (id, ds) =>
      set((s) => ({
        localMode: {
          ...s.localMode,
          downloads: { ...s.localMode.downloads, [id]: ds },
        },
      })),
  },
});
