import i18n from 'i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { initReactI18next } from 'react-i18next';
import enChat from './locales/en/chat.json';
import enCommon from './locales/en/common.json';
import enErrors from './locales/en/errors.json';
import enSettings from './locales/en/settings.json';
import ruChat from './locales/ru/chat.json';
import ruCommon from './locales/ru/common.json';
import ruErrors from './locales/ru/errors.json';
import ruSettings from './locales/ru/settings.json';

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: 'en',
    supportedLngs: ['en', 'ru'],
    defaultNS: 'common',
    ns: ['common', 'chat', 'settings', 'errors'],
    resources: {
      en: {
        common: enCommon,
        chat: enChat,
        settings: enSettings,
        errors: enErrors,
      },
      ru: {
        common: ruCommon,
        chat: ruChat,
        settings: ruSettings,
        errors: ruErrors,
      },
    },
    interpolation: { escapeValue: false },
    react: { useSuspense: false },
  });

export default i18n;
