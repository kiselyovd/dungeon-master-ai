import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type CampaignsSlice, createCampaignsSlice } from './campaigns';
import { type CharCreationSlice, createCharCreationSlice } from './charCreation';
import { type ChatSlice, createChatSlice } from './chat';
import { type CombatSlice, createCombatSlice } from './combat';
import { createJournalSlice, type JournalSlice } from './journal';
import {
  ALL_MODEL_IDS,
  createLocalModeSlice,
  type LocalModeSlice,
  type ModelId,
} from './localMode';
import { createNpcSlice, type NpcSlice } from './npc';
import { createOnboardingSlice, type OnboardingSlice } from './onboarding';
import { createPcSlice, type PcSlice } from './pc';
import { type PersistedSettings, persistStorage } from './persistStorage';
import { createRestorationSlice, type RestorationSlice } from './restoration';
import { createSavesSlice, type SavesSlice } from './saves';
import { createSessionSlice, type SessionSlice } from './session';
import {
  applyD8Migration,
  applyProviderMigration,
  createSettingsSlice,
  type SettingsSlice,
} from './settings';
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
  RestorationSlice &
  CharCreationSlice &
  CampaignsSlice;

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
      ...createRestorationSlice(...a),
      ...createCharCreationSlice(...a),
      ...createCampaignsSlice(...a),
    }),
    {
      name: PERSIST_NAME,
      storage: persistStorage,
      version: 0,
      onRehydrateStorage: () => (_state, error) => {
        if (error) {
          console.error('[persist] rehydration failed:', error);
        }
      },
      partialize: (state): PersistedSettings => ({
        settings: {
          workspace: state.settings.workspace,
          activeProvider: state.settings.activeProvider,
          providers: state.settings.providers,
          uiLanguage: state.settings.uiLanguage,
          narrationLanguage: state.settings.narrationLanguage,
          systemPrompt: state.settings.systemPrompt,
          temperature: state.settings.temperature,
          replicateApiKey: state.settings.replicateApiKey,
          chatPanelWidth: state.settings.chatPanelWidth,
          readingFontSize: state.settings.readingFontSize,
          campaignVisits: state.settings.campaignVisits,
          sceneTransitionsEnabled: state.settings.sceneTransitionsEnabled,
          discoveredCatalogs: state.settings.discoveredCatalogs,
          // M7-DM additions (now persisted)
          imageEnabled: state.settings.imageEnabled,
          imagePreset: state.settings.imagePreset,
          imageStyleLora: state.settings.imageStyleLora,
          videoEnabled: state.settings.videoEnabled,
          videoMode: state.settings.videoMode,
          visionEnabled: state.settings.visionEnabled,
          reasoningEnabled: state.settings.reasoningEnabled,
          reasoningBudget: state.settings.reasoningBudget,
          licenseRestrictedMode: state.settings.licenseRestrictedMode,
          agentMaxRounds: state.settings.agentMaxRounds,
        },
        session: {
          activeCampaignId: state.session.activeCampaignId,
          activeSessionId: state.session.activeSessionId,
          currentScene: state.session.currentScene,
        },
        onboarding: {
          completed: state.onboarding.completed,
          version: state.onboarding.version,
          entry: state.onboarding.entry,
        },
        pc: {
          ...(state.pc.defense ? { defense: state.pc.defense } : {}),
          ...(state.pc.baseSpeedFt !== undefined ? { baseSpeedFt: state.pc.baseSpeedFt } : {}),
          characterId: state.pc.characterId ?? null,
          backstory: state.pc.backstory ?? '',
          ideals: state.pc.ideals ?? '',
          bonds: state.pc.bonds ?? '',
          flaws: state.pc.flaws ?? '',
          spells: state.pc.spells ?? { cantrips: [], level1: [] },
          personalityFlags: state.pc.personalityFlags ?? [],
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
          portraitUrl: state.pc.portraitUrl,
          portraitAssetId: state.pc.portraitAssetId ?? null,
          portraitMediaRef: state.pc.portraitMediaRef ?? null,
        },
        charCreation: {
          rulesetId: state.charCreation.rulesetId,
          defenseChoice: state.charCreation.defenseChoice,
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
          portraitAssetId: state.charCreation.portraitAssetId ?? null,
          portraitMediaRef: state.charCreation.portraitMediaRef ?? null,
          mediaDraftId: state.charCreation.mediaDraftId ?? null,
          draftScope: state.charCreation.draftScope ?? null,
          portraitPrompt: state.charCreation.portraitPrompt,
          activeTab: state.charCreation.activeTab,
        },
      }),
      // Preserve action functions on the in-memory slice objects when the
      // persisted (data-only) snapshot is merged back in.
      merge: (persistedState, currentState) => {
        if (!persistedState || typeof persistedState !== 'object') return currentState;
        const persisted = persistedState as Partial<PersistedSettings>;
        // D8: diagnostic-only rehydration pass over the persisted settings
        // slice (no data-shape change; returns the input unchanged).
        // D.5: reset a legacy `activeProvider:'anthropic'` to unconfigured
        // openai-compat + raise the migration banner.
        const persistedSettings = applyProviderMigration(
          applyD8Migration(persisted.settings ?? {}),
        );
        const savedLocal = persistedSettings.providers?.['local-mistralrs'];
        return {
          ...currentState,
          localMode: {
            ...currentState.localMode,
            ...(savedLocal && ALL_MODEL_IDS.includes(savedLocal.modelPath as ModelId)
              ? {
                  selectedLlm: savedLocal.modelPath as ModelId,
                  contextWindow:
                    Number.isInteger(savedLocal.contextWindow) &&
                    savedLocal.contextWindow >= 512 &&
                    savedLocal.contextWindow <= 65536
                      ? savedLocal.contextWindow
                      : 8192,
                }
              : {}),
          },
          settings: {
            ...currentState.settings,
            ...persistedSettings,
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
