/**
 * Standalone Zustand slice for the Settings -> Local LLM manifest cache.
 *
 * Kept out of the main `useStore` aggregate (M9-DM Task 14) because the
 * surface is small, lives behind a single Settings tab, and has no dependency
 * on character/session/combat state. Tasks 15-19 (HF search) extend this slice
 * with `userManifest` mutations + license-accept flow.
 */

import { create } from 'zustand';

import { cancelOrDeleteModel, fetchLocalLlmManifest, startModelDownload } from '../api/localLlm';
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
  /** Optimistically mark model as queued, then POST to start the download. */
  startDownload: (id: string) => Promise<void>;
  /** Optimistically remove download state, then DELETE to cancel or remove. */
  deleteModel: (id: string) => Promise<void>;
  /** Apply an incoming SSE DownloadEventWire to the local state. */
  applyDownloadEvent: (ev: {
    id: string;
    kind: string;
    bytes_done?: number;
    total_bytes?: number;
    reason?: string;
  }) => void;
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

  async startDownload(id: string) {
    // Optimistic: mark as queued immediately for snappy UX.
    set((s) => {
      const next = new Map(s.downloadStates);
      next.set(id, { state: 'queued' });
      return { downloadStates: next };
    });
    try {
      await startModelDownload(id);
    } catch (e) {
      // Rollback optimistic update on failure.
      set((s) => {
        const next = new Map(s.downloadStates);
        next.set(id, { state: 'error', errorMessage: e instanceof Error ? e.message : String(e) });
        return { downloadStates: next };
      });
    }
  },

  async deleteModel(id: string) {
    // Optimistic: clear download state immediately.
    set((s) => {
      const next = new Map(s.downloadStates);
      next.delete(id);
      const nextInstalled = new Set(s.installedIds);
      nextInstalled.delete(id);
      return { downloadStates: next, installedIds: nextInstalled };
    });
    try {
      await cancelOrDeleteModel(id);
    } catch (e) {
      // Log and reconcile: if no download is in flight, no SSE event will arrive
      // to correct a wrong optimistic removal, so re-fetch the manifest to
      // restore the true backend state.
      console.warn(`[deleteModel] backend DELETE failed for ${id}:`, e);
      void get().loadManifest();
    }
  },

  applyDownloadEvent(ev) {
    set((s) => {
      const next = new Map(s.downloadStates);
      const nextInstalled = new Set(s.installedIds);
      if (ev.kind === 'progress') {
        const progress =
          ev.total_bytes != null && ev.total_bytes > 0 && ev.bytes_done != null
            ? ev.bytes_done / ev.total_bytes
            : undefined;
        const ds: DownloadState = { state: 'downloading' };
        if (progress !== undefined) ds.progress = progress;
        next.set(ev.id, ds);
      } else if (ev.kind === 'completed') {
        next.delete(ev.id);
        nextInstalled.add(ev.id);
      } else if (ev.kind === 'failed') {
        next.set(ev.id, {
          state: 'error',
          ...(ev.reason !== undefined ? { errorMessage: ev.reason } : {}),
        });
      }
      return { downloadStates: next, installedIds: nextInstalled };
    });
  },
}));
