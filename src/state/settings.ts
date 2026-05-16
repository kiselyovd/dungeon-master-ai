import type { StateCreator } from 'zustand';
import type {
  AnthropicConfig,
  LocalMistralRsConfig,
  OpenaiCompatConfig,
  ProviderConfig,
  ProviderKind,
} from './providers';
import type { ImagePreset, ReasoningBudget, VideoMode } from './settingsMigration';

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
  /** Play scene-transition video clips when the active scene changes. */
  sceneTransitionsEnabled: boolean;

  // M7-DM additions (v2 shape). Existing fields above continue to drive M5/M6
  // surfaces; these new fields back the 4-tab Settings UI (Phase D) and the
  // POST /settings v2 endpoint (C.4).
  imageEnabled: boolean;
  imagePreset: ImagePreset;
  imageStyleLora: string | null;
  videoEnabled: boolean;
  videoMode: VideoMode;
  visionEnabled: boolean;
  reasoningEnabled: boolean;
  reasoningBudget: ReasoningBudget;
  licenseRestrictedMode: boolean;
  agentMaxRounds: number;
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
  setSceneTransitionsEnabled: (enabled: boolean) => void;

  // M7-DM
  setImageEnabled: (enabled: boolean) => void;
  setImagePreset: (preset: ImagePreset) => void;
  setImageStyleLora: (lora: string | null) => void;
  setVideoEnabled: (enabled: boolean) => void;
  setVideoMode: (mode: VideoMode) => void;
  setVisionEnabled: (enabled: boolean) => void;
  setReasoningEnabled: (enabled: boolean) => void;
  setReasoningBudget: (budget: ReasoningBudget) => void;
  setLicenseRestrictedMode: (on: boolean) => void;
  setAgentMaxRounds: (n: number) => void;
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
    sceneTransitionsEnabled: true,

    // M7-DM defaults
    imageEnabled: true,
    imagePreset: 'balanced',
    imageStyleLora: null,
    videoEnabled: false,
    videoMode: 'prerecorded',
    visionEnabled: false,
    reasoningEnabled: false,
    reasoningBudget: 'medium',
    licenseRestrictedMode: false,
    agentMaxRounds: 8,

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

    setSceneTransitionsEnabled: (sceneTransitionsEnabled) =>
      set((s) => ({ settings: { ...s.settings, sceneTransitionsEnabled } })),

    setImageEnabled: (imageEnabled) =>
      set((s) => ({ settings: { ...s.settings, imageEnabled } })),
    setImagePreset: (imagePreset) =>
      set((s) => ({ settings: { ...s.settings, imagePreset } })),
    setImageStyleLora: (imageStyleLora) =>
      set((s) => ({ settings: { ...s.settings, imageStyleLora } })),
    setVideoEnabled: (videoEnabled) =>
      set((s) => ({ settings: { ...s.settings, videoEnabled } })),
    setVideoMode: (videoMode) => set((s) => ({ settings: { ...s.settings, videoMode } })),
    setVisionEnabled: (visionEnabled) =>
      set((s) => ({ settings: { ...s.settings, visionEnabled } })),
    setReasoningEnabled: (reasoningEnabled) =>
      set((s) => ({ settings: { ...s.settings, reasoningEnabled } })),
    setReasoningBudget: (reasoningBudget) =>
      set((s) => ({ settings: { ...s.settings, reasoningBudget } })),
    setLicenseRestrictedMode: (licenseRestrictedMode) =>
      set((s) => ({ settings: { ...s.settings, licenseRestrictedMode } })),
    setAgentMaxRounds: (agentMaxRounds) =>
      set((s) => ({ settings: { ...s.settings, agentMaxRounds } })),
  },
});
