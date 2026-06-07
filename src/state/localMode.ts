import type { StateCreator } from 'zustand';
import type { AppState } from './useStore';

// Gemma 4 ids match the backend ModelId serde renames (see manifest.rs); the
// qwen ids are the snake_case backend variant names. Both flow through
// /local-mode/config verbatim.
export type ModelId =
  | 'qwen3_8b'
  | 'gemma4_e2b'
  | 'gemma4_e4b'
  | 'qwen3_5_0_8b'
  | 'qwen3_5_2b'
  | 'qwen3_5_4b'
  | 'qwen3_5_9b'
  | 'sdxl_turbo';
export type VramStrategy = 'auto-swap' | 'keep-both-loaded' | 'disable-image-gen';

/**
 * Session-local descriptor for a custom Hugging Face GGUF entered through
 * the CustomHfRepoModal. When set, it overrides `selectedLlm` at the
 * /settings POST boundary (see api/providers.ts::toWireConfig) so the
 * backend swaps to a ModelId::Custom variant instead of one of the presets.
 * Not persisted across sessions in v1 - user re-enters per restart.
 */
export interface CustomModelDescriptor {
  hf_repo: string;
  gguf_filename: string;
  mmproj_filename?: string;
}

export type RuntimeState =
  | { state: 'off' }
  | { state: 'starting' }
  | { state: 'ready'; port: number }
  | { state: 'failed'; reason: string };

export type DownloadState =
  | { state: 'idle' }
  | { state: 'downloading'; bytesDone: number; totalBytes: number | null }
  | { state: 'completed'; bytesTotal: number }
  | { state: 'failed'; reason: string; authRequired: boolean };

export const ALL_MODEL_IDS: ModelId[] = [
  'qwen3_8b',
  'gemma4_e2b',
  'gemma4_e4b',
  'qwen3_5_0_8b',
  'qwen3_5_2b',
  'qwen3_5_4b',
  'qwen3_5_9b',
  'sdxl_turbo',
];

export interface LocalModeData {
  enabled: boolean;
  selectedLlm: ModelId;
  customLlmOverride: CustomModelDescriptor | null;
  vramStrategy: VramStrategy;
  runtime: { llm: RuntimeState; image: RuntimeState };
  downloads: Record<ModelId, DownloadState>;
}

export interface LocalModeSlice {
  localMode: LocalModeData & {
    setEnabled: (v: boolean) => void;
    selectModel: (id: ModelId) => void;
    setCustomLlmOverride: (desc: CustomModelDescriptor | null) => void;
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
    selectedLlm: 'gemma4_e2b',
    customLlmOverride: null,
    vramStrategy: 'auto-swap',
    runtime: { llm: { state: 'off' }, image: { state: 'off' } },
    downloads: idleDownloads(),
    setEnabled: (v) => set((s) => ({ localMode: { ...s.localMode, enabled: v } })),
    selectModel: (id) => set((s) => ({ localMode: { ...s.localMode, selectedLlm: id } })),
    setCustomLlmOverride: (desc) =>
      set((s) => ({ localMode: { ...s.localMode, customLlmOverride: desc } })),
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
