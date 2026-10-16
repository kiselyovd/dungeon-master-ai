import type { StateCreator } from 'zustand';

/**
 * Persistent pointer to a backend-owned campaign aggregate and its active
 * session. The frontend never invents either identifier.
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
  activate: (campaignId: string, sessionId: string) => void;
  current: () => { campaignId: string; sessionId: string } | null;
  clear: () => void;
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

export const createSessionSlice: StateCreator<SessionSlice, [], [], SessionSlice> = (set, get) => ({
  session: {
    activeCampaignId: null,
    activeSessionId: null,
    loadError: null,
    currentScene: null,
    mapImageUrl: null,

    activate: (campaignId, sessionId) =>
      set((s) => ({
        session: { ...s.session, activeCampaignId: campaignId, activeSessionId: sessionId },
      })),

    current: () => {
      const current = get().session;
      if (current.activeCampaignId === null || current.activeSessionId === null) return null;
      return { campaignId: current.activeCampaignId, sessionId: current.activeSessionId };
    },

    clear: () =>
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
