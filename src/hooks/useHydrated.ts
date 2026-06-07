import { useEffect, useState } from 'react';
import { useStore } from '../state/useStore';

/**
 * Tracks whether the zustand persist middleware has finished its asynchronous
 * rehydration from disk.
 *
 * Before hydration completes the store holds slice defaults (e.g.
 * `onboarding.completed === false`), so anything rendered off those defaults -
 * most painfully the first-run Onboarding modal - flashes on every launch
 * until `getItem` resolves. Gating that UI on this hook keeps it from showing
 * until the real persisted values are in place. (Audit blocker 1.)
 */
export function useHydrated(): boolean {
  const [hydrated, setHydrated] = useState<boolean>(() => useStore.persist.hasHydrated());

  useEffect(() => {
    // Hydration may have finished between the initial render and this effect.
    if (useStore.persist.hasHydrated()) {
      setHydrated(true);
      return;
    }
    return useStore.persist.onFinishHydration(() => setHydrated(true));
  }, []);

  return hydrated;
}
