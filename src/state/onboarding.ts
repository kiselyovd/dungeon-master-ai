import type { StateCreator } from 'zustand';

/**
 * Tiny "have we shown the first-run wizard yet" flag. Persisted via
 * `persistStorage.ts` under `onboarding_completed` so the modal does not
 * pop up on every launch after the user finishes (or skips through) the
 * Onboarding flow.
 *
 * Kept as its own slice rather than folded into `settings` because the
 * lifecycle is different: settings is "what the user picked", onboarding
 * is "has the wizard finished". Splitting keeps the persist payload
 * orthogonal and lets a future "tutorial replay" entry point flip the
 * flag back to false without touching the rest of the settings shape.
 */

export interface OnboardingData {
  completed: boolean;
}

export interface OnboardingActions {
  /** Mark the wizard as finished. Idempotent. */
  complete: () => void;
  /** Reset the flag back to false so the wizard shows again. */
  reset: () => void;
}

export interface OnboardingSlice {
  onboarding: OnboardingData & OnboardingActions;
}

export const createOnboardingSlice: StateCreator<OnboardingSlice, [], [], OnboardingSlice> = (
  set,
) => ({
  onboarding: {
    completed: false,
    complete: () =>
      set((s) => ({
        onboarding: { ...s.onboarding, completed: true },
      })),
    reset: () =>
      set((s) => ({
        onboarding: { ...s.onboarding, completed: false },
      })),
  },
});
