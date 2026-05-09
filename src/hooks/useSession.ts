import { useEffect } from 'react';
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
 * Failures from the messages fetch are swallowed: the user always lands
 * on a usable empty chat even if the backend is unreachable or the
 * session row was wiped between launches.
 */
export function useSession(): void {
  const ensureSession = useStore((s) => s.session.ensureSession);
  const setMessages = useStore((s) => s.chat.setMessages);

  useEffect(() => {
    let cancelled = false;
    const { sessionId } = ensureSession();

    void fetchSessionMessages(sessionId)
      .then((messages) => {
        if (cancelled) return;
        if (messages.length > 0) setMessages(messages);
      })
      .catch(() => {
        // Backend unreachable / session row missing - leave the chat empty.
      });

    return () => {
      cancelled = true;
    };
  }, [ensureSession, setMessages]);
}
