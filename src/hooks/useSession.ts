import { useCallback, useEffect } from 'react';
import { fetchSessionMessages } from '../api/chat';
import { useStore } from '../state/useStore';

/**
 * Mount-time hook that:
 *
 * Pulls the persisted chat history for the explicitly active session via
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
  const campaignId = useStore((s) => s.session.activeCampaignId);
  const sessionId = useStore((s) => s.session.activeSessionId);
  const setMessages = useStore((s) => s.chat.setMessages);
  const setLoadError = useStore((s) => s.session.setLoadError);

  const load = useCallback(
    (signal?: AbortSignal): Promise<void> => {
      if (campaignId === null || sessionId === null) {
        setLoadError('campaign_required');
        return Promise.resolve();
      }
      const generation = useStore.getState().session.transitionGeneration;
      const messagesAtStart = useStore.getState().chat.messages;
      const isCurrent = () =>
        !signal?.aborted &&
        useStore.getState().session.transitionGeneration === generation &&
        useStore.getState().session.activeSessionId === sessionId &&
        useStore.getState().session.activeCampaignId === campaignId;
      return fetchSessionMessages(sessionId)
        .then((messages) => {
          if (!isCurrent()) return;
          if (useStore.getState().chat.messages === messagesAtStart) setMessages(messages);
          setLoadError(null);
        })
        .catch((err) => {
          if (!isCurrent()) return;
          const message = err instanceof Error ? err.message : 'failed to load session messages';
          setLoadError(message);
        });
    },
    [campaignId, sessionId, setMessages, setLoadError],
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
