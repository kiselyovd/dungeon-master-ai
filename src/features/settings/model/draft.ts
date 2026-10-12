import type { SettingsInput } from '../../../api/settings';
import type { ProviderConfig } from '../../../state/providers';

export type SettingsPreferences = Pick<
  SettingsInput,
  | 'sceneTransitionsEnabled'
  | 'imageEnabled'
  | 'imagePreset'
  | 'imageStyleLora'
  | 'videoEnabled'
  | 'videoMode'
  | 'visionEnabled'
  | 'reasoningEnabled'
  | 'reasoningBudget'
  | 'licenseRestrictedMode'
  | 'agentMaxRounds'
>;

export interface SettingsSubmission {
  provider: ProviderConfig;
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string;
  preferences: SettingsPreferences;
}

export function preferenceDraft(s: SettingsInput): SettingsPreferences {
  return {
    sceneTransitionsEnabled: s.sceneTransitionsEnabled,
    imageEnabled: s.imageEnabled,
    imagePreset: s.imagePreset,
    imageStyleLora: s.imageStyleLora,
    videoEnabled: s.videoEnabled,
    videoMode: s.videoMode,
    visionEnabled: s.visionEnabled,
    reasoningEnabled: s.reasoningEnabled,
    reasoningBudget: s.reasoningBudget,
    licenseRestrictedMode: s.licenseRestrictedMode,
    agentMaxRounds: s.agentMaxRounds,
  };
}
