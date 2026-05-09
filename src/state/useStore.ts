import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type ChatSlice, createChatSlice } from './chat';
import { type CombatSlice, createCombatSlice } from './combat';
import { createJournalSlice, type JournalSlice } from './journal';
import { createLocalModeSlice, type LocalModeSlice } from './localMode';
import { createNpcSlice, type NpcSlice } from './npc';
import { createOnboardingSlice, type OnboardingSlice } from './onboarding';
import { createPcSlice, type PcSlice } from './pc';
import { type PersistedSettings, persistStorage } from './persistStorage';
import { createSavesSlice, type SavesSlice } from './saves';
import { createSessionSlice, type SessionSlice } from './session';
import { createSettingsSlice, type SettingsSlice } from './settings';
import { createToolLogSlice, type ToolLogSlice } from './toolLog';

export type AppState = ChatSlice &
  SettingsSlice &
  CombatSlice &
  JournalSlice &
  NpcSlice &
  ToolLogSlice &
  LocalModeSlice &
  SessionSlice &
  OnboardingSlice &
  PcSlice &
  SavesSlice;

const PERSIST_NAME = 'dungeon-master-ai';

export const useStore = create<AppState>()(
  persist(
    (...a) => ({
      ...createChatSlice(...a),
      ...createSettingsSlice(...a),
      ...createCombatSlice(...a),
      ...createJournalSlice(...a),
      ...createNpcSlice(...a),
      ...createToolLogSlice(...a),
      ...createLocalModeSlice(...a),
      ...createSessionSlice(...a),
      ...createOnboardingSlice(...a),
      ...createPcSlice(...a),
      ...createSavesSlice(...a),
    }),
    {
      name: PERSIST_NAME,
      storage: persistStorage,
      version: 0,
      partialize: (state): PersistedSettings => ({
        settings: {
          activeProvider: state.settings.activeProvider,
          providers: state.settings.providers,
          uiLanguage: state.settings.uiLanguage,
          narrationLanguage: state.settings.narrationLanguage,
          systemPrompt: state.settings.systemPrompt,
          temperature: state.settings.temperature,
          replicateApiKey: state.settings.replicateApiKey,
          chatPanelWidth: state.settings.chatPanelWidth,
        },
        session: {
          activeCampaignId: state.session.activeCampaignId,
          activeSessionId: state.session.activeSessionId,
          currentScene: state.session.currentScene,
        },
        onboarding: {
          completed: state.onboarding.completed,
        },
        pc: {
          heroClass: state.pc.heroClass,
        },
      }),
      // Preserve action functions on the in-memory slice objects when the
      // persisted (data-only) snapshot is merged back in.
      merge: (persistedState, currentState) => {
        if (!persistedState || typeof persistedState !== 'object') return currentState;
        const persisted = persistedState as Partial<PersistedSettings>;
        return {
          ...currentState,
          settings: {
            ...currentState.settings,
            ...(persisted.settings ?? {}),
          },
          session: {
            ...currentState.session,
            ...(persisted.session ?? {}),
          },
          onboarding: {
            ...currentState.onboarding,
            ...(persisted.onboarding ?? {}),
          },
          pc: {
            ...currentState.pc,
            ...(persisted.pc ?? {}),
          },
        };
      },
    },
  ),
);
