import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';
import enAgent from './locales/en/agent.json';
import enChat from './locales/en/chat.json';
import enCombat from './locales/en/combat.json';
import enCommon from './locales/en/common.json';
import enErrors from './locales/en/errors.json';
import enJournal from './locales/en/journal.json';
import enNpc from './locales/en/npc.json';
import enSettings from './locales/en/settings.json';
import enUpdater from './locales/en/updater.json';
import ruAgent from './locales/ru/agent.json';
import ruChat from './locales/ru/chat.json';
import ruCombat from './locales/ru/combat.json';
import ruCommon from './locales/ru/common.json';
import ruErrors from './locales/ru/errors.json';
import ruJournal from './locales/ru/journal.json';
import ruNpc from './locales/ru/npc.json';
import ruSettings from './locales/ru/settings.json';
import ruUpdater from './locales/ru/updater.json';

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: 'en',
    supportedLngs: ['en', 'ru'],
    defaultNS: 'common',
    ns: ['common', 'chat', 'settings', 'errors', 'combat', 'journal', 'npc', 'agent', 'updater'],
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
      },
    },
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  });

export default i18n;
