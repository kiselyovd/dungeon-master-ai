/**
 * Standalone Zustand slice for the Settings -> Local LLM manifest cache.
 *
 * Kept out of the main `useStore` aggregate (M9-DM Task 14) because the
 * surface is small, lives behind a single Settings tab, and has no dependency
 * on character/session/combat state. Tasks 15-19 (HF search) extend this slice
 * with `userManifest` mutations + license-accept flow.
 */

import { create } from 'zustand';

import { fetchLocalLlmManifest } from '../api/localLlm';
import {
  type DownloadState,
  type MergedEntry,
  mergeManifests,
  type SystemEntry,
  type UserEntry,
} from './local_llm/manifest';

interface LocalLlmState {
  system: SystemEntry[];
  user: UserEntry[];
  installedIds: Set<string>;
  downloadStates: Map<string, DownloadState>;
  loading: boolean;
  error: string | null;

  loadManifest: () => Promise<void>;
  merged: () => MergedEntry[];
}

type DownloadStateName = DownloadState['state'];

const KNOWN_STATES: readonly DownloadStateName[] = [
  'idle',
  'queued',
  'downloading',
  'verifying',
  'error',
];

function coerceState(raw: string): DownloadStateName {
  return (KNOWN_STATES as readonly string[]).includes(raw) ? (raw as DownloadStateName) : 'idle';
}

export const useLocalLlmStore = create<LocalLlmState>((set, get) => ({
  system: [],
  user: [],
  installedIds: new Set<string>(),
  downloadStates: new Map<string, DownloadState>(),
  loading: false,
  error: null,

  async loadManifest() {
    set({ loading: true, error: null });
    try {
      const res = await fetchLocalLlmManifest();
      const downloadStates = new Map<string, DownloadState>(
        Object.entries(res.download_states).map(([id, ds]) => {
          const ent: DownloadState = { state: coerceState(ds.state) };
          if (ds.progress !== undefined) ent.progress = ds.progress;
          if (ds.errorMessage !== undefined) ent.errorMessage = ds.errorMessage;
          return [id, ent];
        }),
      );
      set({
        system: res.system,
        user: res.user,
        installedIds: new Set(res.installed_ids),
        downloadStates,
        loading: false,
      });
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : String(e) });
    }
  },

  merged() {
    const { system, user, installedIds, downloadStates } = get();
    return mergeManifests(system, user, installedIds, downloadStates);
  },
}));
