import type { StateCreator } from 'zustand';
import type { RestoredSessionProjection } from '../features/saves/model/buildRestoredSession';
import type { ChatSlice } from './chat';
import {
  type CombatSlice,
  emptyCombatProjectionState,
  legacyCombatProjectionState,
} from './combat';
import type { PcSlice } from './pc';
import type { SessionSlice } from './session';

export interface RestorationSlice {
  restoration: {
    apply: (projection: RestoredSessionProjection) => void;
  };
}

type RestorationState = ChatSlice & CombatSlice & PcSlice & SessionSlice & RestorationSlice;

export const createRestorationSlice: StateCreator<RestorationState, [], [], RestorationSlice> = (
  set,
) => ({
  restoration: {
    apply: (projection) =>
      set((state) => ({
        session: {
          ...state.session,
          activeCampaignId: projection.campaignId,
          activeSessionId: projection.sessionId,
          loadError: null,
          currentScene: projection.scene,
          mapImageUrl: null,
        },
        chat: {
          ...state.chat,
          messages: projection.messages,
          chatStreamEvents: [],
          _nextSeq: projection.messages.length,
          streamingAssistant: null,
          streamingReasoning: null,
          isStreaming: false,
          lastError: null,
          abortController: null,
          reasoningStreams: new Map<string, string>(),
        },
        pc: projection.pc === null ? state.pc : { ...state.pc, ...projection.pc },
        combat: {
          ...state.combat,
          ...(projection.combat === null
            ? emptyCombatProjectionState()
            : legacyCombatProjectionState(projection.combat)),
          aoeTemplates: [],
        },
      })),
  },
});
