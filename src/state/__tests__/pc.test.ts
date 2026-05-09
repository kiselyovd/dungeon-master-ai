import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createPcSlice, type PcSlice } from '../pc';

function freshStore() {
  return create<PcSlice>()((...a) => ({
    ...createPcSlice(...a),
  }));
}

describe('PcSlice', () => {
  it('starts with heroClass null so the rest of the app sees "no hero yet"', () => {
    const store = freshStore();
    expect(store.getState().pc.heroClass).toBeNull();
  });

  it('setHeroClass stores the chosen class id verbatim', () => {
    const store = freshStore();
    store.getState().pc.setHeroClass('wizard');
    expect(store.getState().pc.heroClass).toBe('wizard');
  });

  it('setHeroClass(null) clears the previously-chosen class', () => {
    const store = freshStore();
    store.getState().pc.setHeroClass('rogue');
    store.getState().pc.setHeroClass(null);
    expect(store.getState().pc.heroClass).toBeNull();
  });
});
