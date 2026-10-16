import type { StateCreator } from 'zustand';
import type { RestoredSessionProjection } from '../features/saves/model/buildRestoredSession';
import type { ChatSlice } from './chat';
import { type CombatSlice, combatProjectionState, emptyCombatProjectionState } from './combat';
import type { JournalSlice } from './journal';
import type { NpcSlice } from './npc';
import type { PcSlice } from './pc';
import type { SessionSlice } from './session';

export interface RestorationSlice {
  restoration: {
    apply: (projection: RestoredSessionProjection) => void;
  };
}

type RestorationState = ChatSlice &
  CombatSlice &
  PcSlice &
  SessionSlice &
  JournalSlice &
  NpcSlice &
  RestorationSlice;

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
        pc: { ...state.pc, ...projection.pc },
        combat: {
          ...state.combat,
          ...(projection.combat === null
            ? emptyCombatProjectionState()
            : combatProjectionState(projection.combat)),
          aoeTemplates: [],
        },
        journal: { ...state.journal, entries: projection.journal },
        npcs: {
          ...state.npcs,
          records: Object.fromEntries(projection.npcs.map((npc) => [npc.name, npc])),
        },
      })),
  },
});
