import type { StateCreator } from 'zustand';

export type Language = 'en' | 'ru';

export interface SettingsSlice {
  settings: {
    anthropicApiKey: string | undefined;
    uiLanguage: Language;
    narrationLanguage: Language;
    setApiKey: (key: string | undefined) => void;
    setUiLanguage: (lang: Language) => void;
    setNarrationLanguage: (lang: Language) => void;
  };
}

export const createSettingsSlice: StateCreator<SettingsSlice, [], [], SettingsSlice> = (set) => ({
  settings: {
    anthropicApiKey: undefined,
    uiLanguage: 'en',
    narrationLanguage: 'en',
    setApiKey: (anthropicApiKey) => set((s) => ({ settings: { ...s.settings, anthropicApiKey } })),
    setUiLanguage: (uiLanguage) => set((s) => ({ settings: { ...s.settings, uiLanguage } })),
    setNarrationLanguage: (narrationLanguage) =>
      set((s) => ({ settings: { ...s.settings, narrationLanguage } })),
  },
});
