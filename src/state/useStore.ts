import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type ChatSlice, createChatSlice } from './chat';
import { type CombatSlice, createCombatSlice } from './combat';
import { type PersistedSettings, persistStorage } from './persistStorage';
import { createSettingsSlice, type SettingsSlice } from './settings';

export type AppState = ChatSlice & SettingsSlice & CombatSlice;

const PERSIST_NAME = 'dungeon-master-ai';

export const useStore = create<AppState>()(
  persist(
    (...a) => ({
      ...createChatSlice(...a),
      ...createSettingsSlice(...a),
      ...createCombatSlice(...a),
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
        },
      }),
      // Preserve action functions on the in-memory `settings` object when the
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
        };
      },
    },
  ),
);
