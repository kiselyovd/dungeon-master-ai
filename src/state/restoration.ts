import type { StateCreator } from 'zustand';
import type { RestoredSessionProjection } from './campaignProjection';
import type { CampaignsSlice } from './campaigns';
import type { CharCreationSlice } from './charCreation';
import { EMPTY_DRAFT } from './charCreation';
import type { ChatSlice } from './chat';
import { type CombatSlice, combatProjectionState, emptyCombatProjectionState } from './combat';
import type { JournalSlice } from './journal';
import type { NpcSlice } from './npc';
import { emptyPcData, type PcSlice } from './pc';
import type { SavesSlice } from './saves';
import type { SessionSlice } from './session';
import type { ToolLogSlice } from './toolLog';

export interface RestorationSlice {
  restoration: {
    apply: (projection: RestoredSessionProjection) => void;
    clear: () => void;
  };
}

type RestorationState = ChatSlice &
  CampaignsSlice &
  CharCreationSlice &
  SavesSlice &
  ToolLogSlice &
  CombatSlice &
  PcSlice &
  SessionSlice &
  JournalSlice &
  NpcSlice &
  RestorationSlice;

export const createRestorationSlice: StateCreator<RestorationState, [], [], RestorationSlice> = (
  set,
  get,
) => {
  const apply = (input: RestoredSessionProjection | null) => {
    const projection = input ?? {
      campaignId: '',
      sessionId: '',
      revision: 0,
      messages: [],
      pc: {},
      scene: null,
      combat: null,
      journal: [],
      npcs: [],
    };
    get().chat.abortController?.abort();
    set((state) => ({
      campaigns: {
        ...state.campaigns,
        active:
          input === null
            ? null
            : (projection.campaign ??
              (state.campaigns.active?.id === projection.campaignId
                ? { ...state.campaigns.active, revision: projection.revision }
                : null)),
        loading: false,
        errorCode: null,
      },
      charCreation: projection.preserveDraft
        ? state.charCreation
        : {
            ...state.charCreation,
            ...structuredClone(EMPTY_DRAFT),
            activeTab: 'class',
            isAssisting: false,
          },
      saves: {
        ...state.saves,
        list: [],
        selectedSaveId: null,
        isOpen: false,
        lastSaveError: null,
        lastQuickSaveAt: null,
      },
      toolLog: { ...state.toolLog, entries: [], isOpen: false },
      session: {
        ...state.session,
        mutationId: null,
        pendingRestore: null,
        activeCampaignId: input === null ? null : projection.campaignId,
        activeSessionId: input === null ? null : projection.sessionId,
        transitionGeneration: state.session.transitionGeneration + 1,
        loadError: null,
        currentScene: projection.scene,
        adventure: projection.adventure ?? null,
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
      pc: { ...state.pc, ...emptyPcData(), ...projection.pc },
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
    }));
  };
  return { restoration: { apply, clear: () => apply(null) } };
};
