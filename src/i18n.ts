import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';
import enAgent from './locales/en/agent.json';
import enCharacter from './locales/en/character.json';
import enChat from './locales/en/chat.json';
import enCombat from './locales/en/combat.json';
import enCommon from './locales/en/common.json';
import enErrors from './locales/en/errors.json';
import enJournal from './locales/en/journal.json';
import enLocalMode from './locales/en/local_mode.json';
import enNpc from './locales/en/npc.json';
import enOnboarding from './locales/en/onboarding.json';
import enSaves from './locales/en/saves.json';
import enSettings from './locales/en/settings.json';
import enUpdater from './locales/en/updater.json';
import ruAgent from './locales/ru/agent.json';
import ruCharacter from './locales/ru/character.json';
import ruChat from './locales/ru/chat.json';
import ruCombat from './locales/ru/combat.json';
import ruCommon from './locales/ru/common.json';
import ruErrors from './locales/ru/errors.json';
import ruJournal from './locales/ru/journal.json';
import ruLocalMode from './locales/ru/local_mode.json';
import ruNpc from './locales/ru/npc.json';
import ruOnboarding from './locales/ru/onboarding.json';
import ruSaves from './locales/ru/saves.json';
import ruSettings from './locales/ru/settings.json';
import ruUpdater from './locales/ru/updater.json';

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: 'en',
    supportedLngs: ['en', 'ru'],
    defaultNS: 'common',
    ns: [
      'common',
      'chat',
      'settings',
      'errors',
      'combat',
      'journal',
      'npc',
      'agent',
      'updater',
      'local_mode',
      'onboarding',
      'saves',
      'character',
    ],
    resources: {
      en: {
        common: enCommon,
        chat: enChat,
        settings: enSettings,
        errors: enErrors,
        combat: enCombat,
        journal: enJournal,
        npc: enNpc,
        agent: enAgent,
        updater: enUpdater,
        local_mode: enLocalMode,
        onboarding: enOnboarding,
        saves: enSaves,
        character: enCharacter,
      },
      ru: {
        common: ruCommon,
        chat: ruChat,
        settings: ruSettings,
        errors: ruErrors,
        combat: ruCombat,
        journal: ruJournal,
        npc: ruNpc,
        agent: ruAgent,
        updater: ruUpdater,
        local_mode: ruLocalMode,
        onboarding: ruOnboarding,
        saves: ruSaves,
        character: ruCharacter,
      },
    },
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  });

export default i18n;
