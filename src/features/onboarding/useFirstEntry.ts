import { useEffect, useRef } from 'react';
import type { OnboardingEntry } from '../../state/onboarding';

/** Resume a saved guest entry once per boot, without forcing model setup. */
export function useFirstEntry(
  hydrated: boolean,
  entry: OnboardingEntry | null,
  onGuest: () => void,
) {
  const resolved = useRef(false);
  useEffect(() => {
    if (!hydrated || resolved.current) return;
    resolved.current = true;
    if (entry === 'guest') onGuest();
  }, [hydrated, entry, onGuest]);
}
