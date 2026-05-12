import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type CharCreationSlice, createCharCreationSlice } from './charCreation';
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
  SavesSlice &
  CharCreationSlice;

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
      ...createCharCreationSlice(...a),
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
          sceneTransitionsEnabled: state.settings.sceneTransitionsEnabled,
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
          name: state.pc.name,
          race: state.pc.race,
          subclass: state.pc.subclass,
          background: state.pc.background,
          alignment: state.pc.alignment,
          level: state.pc.level,
          experience: state.pc.experience,
          experienceNext: state.pc.experienceNext,
          hp: state.pc.hp,
          hpMax: state.pc.hpMax,
          ac: state.pc.ac,
          initiative: state.pc.initiative,
          speedFt: state.pc.speedFt,
          proficiencyBonus: state.pc.proficiencyBonus,
          abilities: state.pc.abilities,
          savingThrowProfs: state.pc.savingThrowProfs,
          skillProfs: state.pc.skillProfs,
          inventory: state.pc.inventory,
        },
        charCreation: {
          classId: state.charCreation.classId,
          subclassId: state.charCreation.subclassId,
          raceId: state.charCreation.raceId,
          subraceId: state.charCreation.subraceId,
          backgroundId: state.charCreation.backgroundId,
          abilityMethod: state.charCreation.abilityMethod,
          abilities: state.charCreation.abilities,
          abilityRollHistory: state.charCreation.abilityRollHistory,
          pointBuyRemaining: state.charCreation.pointBuyRemaining,
          skillProfs: state.charCreation.skillProfs,
          spells: state.charCreation.spells,
          equipmentMode: state.charCreation.equipmentMode,
          equipmentSlots: state.charCreation.equipmentSlots,
          equipmentInventory: state.charCreation.equipmentInventory,
          goldRemaining: state.charCreation.goldRemaining,
          personalityFlags: state.charCreation.personalityFlags,
          ideals: state.charCreation.ideals,
          bonds: state.charCreation.bonds,
          flaws: state.charCreation.flaws,
          backstory: state.charCreation.backstory,
          name: state.charCreation.name,
          alignment: state.charCreation.alignment,
          portraitUrl: state.charCreation.portraitUrl,
          portraitPrompt: state.charCreation.portraitPrompt,
          activeTab: state.charCreation.activeTab,
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
          charCreation: {
            ...currentState.charCreation,
            ...(persisted.charCreation ?? {}),
          },
        };
      },
    },
  ),
);
