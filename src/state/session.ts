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

/**
 * Snapshot of the current scene shown in the titlebar centre slot. The
 * step counter advances on each major narrative beat (currently driven
 * manually by the agent loop; in M5+ the orchestrator will own it).
 * `null` means no scene is active and the pill is hidden.
 */
export interface CurrentScene {
  name: string;
  stepCounter: number;
}

export interface SessionData {
  activeCampaignId: string | null;
  activeSessionId: string | null;
  /**
   * Last error encountered while loading session messages. Surfaced as a
   * retry-bar in the chat panel; cleared on a successful refetch.
   */
  loadError: string | null;
  /** Active scene shown in the titlebar centre. Null when no scene is set. */
  currentScene: CurrentScene | null;
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
  /** Set or clear the message-load error shown in the chat retry bar. */
  setLoadError: (message: string | null) => void;
  /** Replace the whole scene snapshot, or clear it with `null`. */
  setCurrentScene: (scene: CurrentScene | null) => void;
  /** +1 the active scene's step counter. No-op when no scene is set. */
  incrementScene: () => void;
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
    loadError: null,
    currentScene: null,

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

    setLoadError: (message) =>
      set((s) => ({
        session: { ...s.session, loadError: message },
      })),

    setCurrentScene: (scene) =>
      set((s) => ({
        session: { ...s.session, currentScene: scene },
      })),

    incrementScene: () => {
      const current = get().session.currentScene;
      if (current === null) return;
      set((s) => ({
        session: {
          ...s.session,
          currentScene: { name: current.name, stepCounter: current.stepCounter + 1 },
        },
      }));
    },
  },
});
