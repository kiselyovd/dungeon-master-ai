import type { StateCreator } from 'zustand';
import type {
  AnthropicConfig,
  LocalMistralRsConfig,
  OpenaiCompatConfig,
  ProviderConfig,
  ProviderKind,
} from './providers';

export type Language = 'en' | 'ru';

export interface ProvidersMap {
  anthropic: AnthropicConfig | null;
  'openai-compat': OpenaiCompatConfig | null;
  'local-mistralrs': LocalMistralRsConfig | null;
}

/**
 * Persisted half of the settings slice. Kept as a separate type so the
 * persist middleware's `partialize` can pick exactly these fields without
 * dragging in the action functions, which are not serialisable.
 */
export interface SettingsData {
  activeProvider: ProviderKind;
  providers: ProvidersMap;
  uiLanguage: Language;
  narrationLanguage: Language;
}

export interface SettingsActions {
  setActiveProvider: (kind: ProviderKind) => void;
  setProviderConfig: (config: ProviderConfig) => void;
  clearProviderConfig: (kind: ProviderKind) => void;
  setUiLanguage: (lang: Language) => void;
  setNarrationLanguage: (lang: Language) => void;
}

export interface SettingsSlice {
  settings: SettingsData & SettingsActions;
}

const DEFAULT_PROVIDERS: ProvidersMap = {
  anthropic: null,
  'openai-compat': null,
  'local-mistralrs': null,
};

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  settings: {
    activeProvider: 'anthropic',
    providers: DEFAULT_PROVIDERS,
    uiLanguage: 'en',
    narrationLanguage: 'en',

    setActiveProvider: (activeProvider) =>
      set((s) => ({ settings: { ...s.settings, activeProvider } })),

    setProviderConfig: (config) =>
      set((s) => ({
        settings: {
          ...s.settings,
          providers: { ...s.settings.providers, [config.kind]: config },
        },
      })),

    clearProviderConfig: (kind) =>
      set((s) => ({
        settings: {
          ...s.settings,
          providers: { ...s.settings.providers, [kind]: null },
        },
      })),

    setUiLanguage: (uiLanguage) => set((s) => ({ settings: { ...s.settings, uiLanguage } })),

    setNarrationLanguage: (narrationLanguage) =>
      set((s) => ({ settings: { ...s.settings, narrationLanguage } })),
  },
});
