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
 * Min/max bounds for the user-resizable chat panel (in CSS pixels). Exposed
 * as named constants so the drag handle, keyboard nudge, and persisted-value
 * sanitiser all share a single source of truth.
 */
export const MIN_CHAT_WIDTH = 360;
export const MAX_CHAT_WIDTH = 640;
export const DEFAULT_CHAT_WIDTH = 480;

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
  // Model tab (M3)
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string | null;
  // P3.21 - chat panel width (px). Clamped to [MIN_CHAT_WIDTH, MAX_CHAT_WIDTH]
  // by setChatPanelWidth and by the persistStorage sanitiser on load.
  chatPanelWidth: number;
}

export interface SettingsActions {
  setActiveProvider: (kind: ProviderKind) => void;
  setProviderConfig: (config: ProviderConfig) => void;
  clearProviderConfig: (kind: ProviderKind) => void;
  setUiLanguage: (lang: Language) => void;
  setNarrationLanguage: (lang: Language) => void;
  setSystemPrompt: (prompt: string) => void;
  setTemperature: (temp: number) => void;
  setReplicateApiKey: (key: string | null) => void;
  setChatPanelWidth: (width: number) => void;
}

function clampChatWidth(width: number): number {
  if (!Number.isFinite(width)) return DEFAULT_CHAT_WIDTH;
  if (width < MIN_CHAT_WIDTH) return MIN_CHAT_WIDTH;
  if (width > MAX_CHAT_WIDTH) return MAX_CHAT_WIDTH;
  return width;
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
    systemPrompt: '',
    temperature: 0.7,
    replicateApiKey: null,
    chatPanelWidth: DEFAULT_CHAT_WIDTH,

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

    setSystemPrompt: (systemPrompt) => set((s) => ({ settings: { ...s.settings, systemPrompt } })),

    setTemperature: (temperature) => set((s) => ({ settings: { ...s.settings, temperature } })),

    setReplicateApiKey: (replicateApiKey) =>
      set((s) => ({ settings: { ...s.settings, replicateApiKey } })),

    setChatPanelWidth: (width) =>
      set((s) => ({
        settings: { ...s.settings, chatPanelWidth: clampChatWidth(width) },
      })),
  },
});
