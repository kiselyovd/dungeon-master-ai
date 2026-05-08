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
  it('starts disabled with qwen3_5_4b selected and auto-swap', () => {
    const s = freshStore().getState().localMode;
    expect(s.enabled).toBe(false);
    expect(s.selectedLlm).toBe('qwen3_5_4b');
    expect(s.vramStrategy).toBe('auto-swap');
    expect(s.runtime.llm.state).toBe('off');
    expect(s.runtime.image.state).toBe('off');
    expect(s.downloads.qwen3_5_4b.state).toBe('idle');
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
});
