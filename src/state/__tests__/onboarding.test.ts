import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import { createOnboardingSlice, type OnboardingSlice } from '../onboarding';

function freshStore() {
  return create<OnboardingSlice>()((...a) => ({
    ...createOnboardingSlice(...a),
  }));
}

describe('OnboardingSlice', () => {
  it('starts with completed=false so the wizard renders on first launch', () => {
    const store = freshStore();
    expect(store.getState().onboarding.completed).toBe(false);
  });

  it('complete flips the flag to true and is idempotent', () => {
    const store = freshStore();
    store.getState().onboarding.complete();
    expect(store.getState().onboarding.completed).toBe(true);
    store.getState().onboarding.complete();
    expect(store.getState().onboarding.completed).toBe(true);
  });

  it('reset flips the flag back to false so the wizard can replay', () => {
    const store = freshStore();
    store.getState().onboarding.complete();
    store.getState().onboarding.reset();
    expect(store.getState().onboarding.completed).toBe(false);
  });
});
