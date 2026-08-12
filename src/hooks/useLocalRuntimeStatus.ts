import { useEffect } from 'react';
import { fetchLocalRuntimeStatus } from '../api/localRuntime';
import { useStore } from '../state/useStore';

const POLL_INTERVAL_MS = 5_000;

export function useLocalRuntimeStatus(enabled: boolean) {
  const setRuntimeStatus = useStore((s) => s.localMode.setRuntimeStatus);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    let requestInFlight = false;
    const tick = async () => {
      if (requestInFlight) return;
      requestInFlight = true;
      try {
        const data = await fetchLocalRuntimeStatus();
        if (!cancelled) setRuntimeStatus(data);
      } catch {
        // intentional: backend may not be up yet, retry on the next tick.
      } finally {
        requestInFlight = false;
      }
    };
    void tick();
    const id = setInterval(() => {
      void tick();
    }, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [enabled, setRuntimeStatus]);
}
