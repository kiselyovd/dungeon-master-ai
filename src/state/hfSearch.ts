/**
 * Zustand slice for the Settings -> Local LLM -> Search Hugging Face panel.
 *
 * Owns the search input + filter params, the current result list, and the
 * loading/error flags. Kept separate from `useLocalLlmStore` because the HF
 * search panel is independent of the installed-manifest cache and has its
 * own request lifecycle (M9-DM Task 19).
 *
 * `repollGatedCards` exists for the license-accept flow (window `focus`
 * event re-poll). It is a no-op in M9; consumers re-run `runSearch()` to
 * pick up a freshly accepted license. The hook is kept so the component
 * does not need to change when M10 wires per-card re-checks.
 */

import { create } from 'zustand';
import { search as apiSearch, type HfModel, type SearchParams } from '../api/hf';

interface HfSearchState {
  params: SearchParams;
  results: HfModel[];
  loading: boolean;
  error: string | null;

  setParam: <K extends keyof SearchParams>(key: K, value: SearchParams[K]) => void;
  setQuery: (q: string) => void;
  runSearch: () => Promise<void>;
  repollGatedCards: () => Promise<void>;
}

export const useHfSearchStore = create<HfSearchState>((set, get) => ({
  params: { q: '', sort: 'downloads' },
  results: [],
  loading: false,
  error: null,

  setParam(key, value) {
    set((s) => ({ params: { ...s.params, [key]: value } }));
  },

  setQuery(q) {
    set((s) => ({ params: { ...s.params, q } }));
  },

  async runSearch() {
    const { params } = get();
    if (!params.q.trim()) {
      set({ results: [], error: null });
      return;
    }
    set({ loading: true, error: null });
    try {
      const results = await apiSearch(params);
      set({ results, loading: false });
    } catch (e) {
      set({ loading: false, error: e instanceof Error ? e.message : String(e) });
    }
  },

  async repollGatedCards() {
    // No-op in M9. Consumers re-run `runSearch()` to refresh a card whose
    // gated license was just accepted. The hook is kept so HfSearch can
    // register a window `focus` listener today without a follow-up component
    // change when M10 introduces per-card license re-checks.
  },
}));
