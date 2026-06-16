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
  /**
   * Data URL of the latest agent-generated scene image, painted as the VTT
   * map background. Null when no image has been generated yet. Ephemeral by
   * design - kept out of the persist whitelist so a multi-MB base64 PNG never
   * lands in localStorage; it repaints on the next `generate_image`. [M11]
   */
  mapImageUrl: string | null;
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
  /** Set or clear the VTT map background image (data URL). */
  setMapImage: (dataUrl: string | null) => void;
}

export interface SessionSlice {
  session: SessionData & SessionActions;
}

function newUuid(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }
  if (typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function') {
    // WebCrypto present but no randomUUID: build an RFC4122 v4 from CSPRNG bytes.
    // Never Math.random - these ids key persisted campaign/session state.
    const bytes = Array.from(crypto.getRandomValues(new Uint8Array(16)), (x, i) => {
      if (i === 6) return (x & 0x0f) | 0x40; // version 4
      if (i === 8) return (x & 0x3f) | 0x80; // variant 10x
      return x;
    });
    const h = bytes.map((x) => x.toString(16).padStart(2, '0')).join('');
    return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
  }
  throw new Error(
    'newUuid: no secure crypto source (crypto.randomUUID / getRandomValues) available',
  );
}

export const createSessionSlice: StateCreator<SessionSlice, [], [], SessionSlice> = (set, get) => ({
  session: {
    activeCampaignId: null,
    activeSessionId: null,
    loadError: null,
    currentScene: null,
    mapImageUrl: null,

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

    setMapImage: (dataUrl) =>
      set((s) => ({
        session: { ...s.session, mapImageUrl: dataUrl },
      })),
  },
});
