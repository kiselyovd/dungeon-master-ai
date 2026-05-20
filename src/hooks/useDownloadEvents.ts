/**
 * Mount-once hook that subscribes to the backend SSE download event stream and
 * dispatches each event into the localLlm Zustand slice.
 *
 * Designed for use in `ModelSelector` (or another single top-level component)
 * so there is exactly one EventSource connection for the lifetime of the
 * Settings panel. The subscription is torn down on unmount.
 */

import { useEffect } from 'react';
import { subscribeDownloadEvents } from '../api/localLlm';
import { useLocalLlmStore } from '../state/localLlm';

export function useDownloadEvents(): void {
  const applyDownloadEvent = useLocalLlmStore((s) => s.applyDownloadEvent);

  useEffect(() => {
    let cancel: (() => void) | undefined;
    let unmounted = false;

    subscribeDownloadEvents((ev) => {
      applyDownloadEvent(ev);
    })
      .then((cancelFn) => {
        if (unmounted) {
          // Component unmounted before the async subscription resolved; tear it
          // down immediately so the EventSource does not leak.
          cancelFn();
        } else {
          cancel = cancelFn;
        }
      })
      .catch((err) => {
        console.warn('[useDownloadEvents] failed to subscribe:', err);
      });

    return () => {
      unmounted = true;
      cancel?.();
    };
    // applyDownloadEvent is a Zustand store method and is a stable reference
    // across renders; listing it here satisfies the exhaustive-deps rule without
    // causing re-subscription churn.
  }, [applyDownloadEvent]);
}
