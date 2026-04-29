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

export interface SettingsSlice {
  settings: {
    activeProvider: ProviderKind;
    providers: ProvidersMap;
    uiLanguage: Language;
    narrationLanguage: Language;

    setActiveProvider: (kind: ProviderKind) => void;
    setProviderConfig: (config: ProviderConfig) => void;
    clearProviderConfig: (kind: ProviderKind) => void;
    setUiLanguage: (lang: Language) => void;
    setNarrationLanguage: (lang: Language) => void;
    /** Replace the entire settings sub-tree (used by the persist hydrate path). */
    hydrate: (next: HydrateInput) => void;
  };
}

export interface HydrateInput {
  activeProvider?: ProviderKind;
  providers?: Partial<ProvidersMap>;
  uiLanguage?: Language;
  narrationLanguage?: Language;
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

    hydrate: (next) =>
      set((s) => ({
        settings: {
          ...s.settings,
          ...(next.activeProvider !== undefined ? { activeProvider: next.activeProvider } : {}),
          ...(next.uiLanguage !== undefined ? { uiLanguage: next.uiLanguage } : {}),
          ...(next.narrationLanguage !== undefined
            ? { narrationLanguage: next.narrationLanguage }
            : {}),
          providers: {
            ...DEFAULT_PROVIDERS,
            ...s.settings.providers,
            ...(next.providers ?? {}),
          },
        },
      })),
  },
});
