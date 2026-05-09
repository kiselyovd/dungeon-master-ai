import type { StateCreator } from 'zustand';

/**
 * Persistent active campaign + session pointer. v1 keeps a single
 * active session per campaign (linear save model per the M2 design
 * decisions). v2 adds branching saves on top of the same shape.
 *
 * The IDs are UUIDs lazily minted on first launch by `ensureSession`,
 * then persisted via the split-storage adapter so a restart picks the
 * same session and the chat history rehydrates from
 * `/sessions/{id}/messages`.
 */
export interface SessionData {
  activeCampaignId: string | null;
  activeSessionId: string | null;
}

export interface SessionActions {
  setActiveSession: (campaignId: string, sessionId: string) => void;
  /**
   * Mint a fresh `(campaignId, sessionId)` pair if either is missing.
   * Returns the resolved pair so callers can use it without an extra
   * store read. Idempotent when both IDs are already set.
   */
  ensureSession: () => { campaignId: string; sessionId: string };
  /** Forget the current session - next `ensureSession` mints a new pair. */
  clearSession: () => void;
}

export interface SessionSlice {
  session: SessionData & SessionActions;
}

function newUuid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  // Fallback for environments without WebCrypto. Not RFC4122 strict;
  // good enough as a stable opaque key.
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 14)}`;
}

export const createSessionSlice: StateCreator<SessionSlice, [], [], SessionSlice> = (set, get) => ({
  session: {
    activeCampaignId: null,
    activeSessionId: null,

    setActiveSession: (campaignId, sessionId) =>
      set((s) => ({
        session: { ...s.session, activeCampaignId: campaignId, activeSessionId: sessionId },
      })),

    ensureSession: () => {
      const current = get().session;
      const campaignId = current.activeCampaignId ?? newUuid();
      const sessionId = current.activeSessionId ?? newUuid();
      if (campaignId !== current.activeCampaignId || sessionId !== current.activeSessionId) {
        set((s) => ({
          session: { ...s.session, activeCampaignId: campaignId, activeSessionId: sessionId },
        }));
      }
      return { campaignId, sessionId };
    },

    clearSession: () =>
      set((s) => ({
        session: { ...s.session, activeCampaignId: null, activeSessionId: null },
      })),
  },
});
