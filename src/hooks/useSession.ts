import { useCallback, useEffect } from 'react';
import { fetchSessionMessages } from '../api/chat';
import { useStore } from '../state/useStore';

/**
 * Mount-time hook that:
 *
 * 1. Lazily mints `(activeCampaignId, activeSessionId)` if neither is set
 *    (first launch).
 * 2. Pulls the persisted chat history for the active session via
 *    `GET /sessions/{id}/messages` and seeds the chat slice so the UI
 *    rehydrates after a restart (M4.5 release-checklist line 54).
 *
 * Failures surface via `state.session.loadError` so the chat panel can
 * render a retry bar instead of leaving the user stranded with an empty
 * conversation when the backend is briefly unreachable.
 */
export interface UseSessionReturn {
  /** Re-run the message fetch. Clears `loadError` on success. */
  refetch: () => void;
}

export function useSession(): UseSessionReturn {
  const ensureSession = useStore((s) => s.session.ensureSession);
  const setMessages = useStore((s) => s.chat.setMessages);
  const setLoadError = useStore((s) => s.session.setLoadError);

  const load = useCallback(
    (signal?: AbortSignal): Promise<void> => {
      const { sessionId } = ensureSession();
      return fetchSessionMessages(sessionId)
        .then((messages) => {
          if (signal?.aborted) return;
          if (messages.length > 0) setMessages(messages);
          setLoadError(null);
        })
        .catch((err) => {
          if (signal?.aborted) return;
          const message = err instanceof Error ? err.message : 'failed to load session messages';
          setLoadError(message);
        });
    },
    [ensureSession, setMessages, setLoadError],
  );

  useEffect(() => {
    const controller = new AbortController();
    void load(controller.signal);
    return () => {
      controller.abort();
    };
  }, [load]);

  const refetch = useCallback(() => {
    void load();
  }, [load]);

  return { refetch };
}
