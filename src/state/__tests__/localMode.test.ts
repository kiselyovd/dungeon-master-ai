import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createLocalModeSlice, type LocalModeSlice } from '../localMode';

function freshStore() {
  return create<LocalModeSlice>()((set, get, store) => ({
    ...createLocalModeSlice(
      set as Parameters<typeof createLocalModeSlice>[0],
      get as Parameters<typeof createLocalModeSlice>[1],
      store as Parameters<typeof createLocalModeSlice>[2],
    ),
  }));
}

describe('LocalModeSlice', () => {
  it('starts disabled with gemma4_e2b selected and auto-swap', () => {
    const s = freshStore().getState().localMode;
    expect(s.enabled).toBe(false);
    expect(s.selectedLlm).toBe('gemma4_e2b');
    expect(s.vramStrategy).toBe('auto-swap');
    expect(s.runtime.llm.state).toBe('off');
    expect(s.runtime.image.state).toBe('off');
    expect(s.downloads.gemma4_e2b.state).toBe('idle');
  });

  it('setEnabled toggles enabled flag', () => {
    const store = freshStore();
    store.getState().localMode.setEnabled(true);
    expect(store.getState().localMode.enabled).toBe(true);
  });

  it('selectModel switches selectedLlm', () => {
    const store = freshStore();
    store.getState().localMode.selectModel('qwen3_5_2b');
    expect(store.getState().localMode.selectedLlm).toBe('qwen3_5_2b');
  });

  it('setVramStrategy persists the choice', () => {
    const store = freshStore();
    store.getState().localMode.setVramStrategy('disable-image-gen');
    expect(store.getState().localMode.vramStrategy).toBe('disable-image-gen');
  });

  it('setDownloadState updates per-model state', () => {
    const store = freshStore();
    store
      .getState()
      .localMode.setDownloadState('sdxl_turbo', { state: 'completed', bytesTotal: 7_000_000_000 });
    expect(store.getState().localMode.downloads.sdxl_turbo).toEqual({
      state: 'completed',
      bytesTotal: 7_000_000_000,
    });
  });

  it('setRuntimeStatus replaces both runtime entries', () => {
    const store = freshStore();
    store.getState().localMode.setRuntimeStatus({
      llm: { state: 'ready', port: 37000 },
      image: { state: 'failed', reason: 'crashed' },
    });
    const r = store.getState().localMode.runtime;
    expect(r.llm).toEqual({ state: 'ready', port: 37000 });
    expect(r.image).toEqual({ state: 'failed', reason: 'crashed' });
  });

  it('customLlmOverride starts null', () => {
    expect(freshStore().getState().localMode.customLlmOverride).toBeNull();
  });

  it('setCustomLlmOverride stores and clears the descriptor', () => {
    const store = freshStore();
    store.getState().localMode.setCustomLlmOverride({
      hf_repo: 'Qwen/Qwen2.5-VL-7B-Instruct-GGUF',
      gguf_filename: 'qwen2.5-vl-7b-instruct-q4_k_m.gguf',
      mmproj_filename: 'mmproj-qwen2.5-vl-7b-f16.gguf',
    });
    expect(store.getState().localMode.customLlmOverride).toEqual({
      hf_repo: 'Qwen/Qwen2.5-VL-7B-Instruct-GGUF',
      gguf_filename: 'qwen2.5-vl-7b-instruct-q4_k_m.gguf',
      mmproj_filename: 'mmproj-qwen2.5-vl-7b-f16.gguf',
    });
    store.getState().localMode.setCustomLlmOverride(null);
    expect(store.getState().localMode.customLlmOverride).toBeNull();
  });
});
