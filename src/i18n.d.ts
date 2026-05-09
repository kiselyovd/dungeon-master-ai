/**
 * Type-augment react-i18next so `t('foo:bar')` is autocompleted and typo-checked.
 * The English bundle is the source of truth - if RU drifts, type-check is fine
 * (resources are merged), but missing keys are caught at compile time wherever
 * the typed `t` is consumed.
 */

import 'react-i18next';

import enCharacter from './locales/en/character.json';
import enChat from './locales/en/chat.json';
import enCommon from './locales/en/common.json';
import enErrors from './locales/en/errors.json';
import enOnboarding from './locales/en/onboarding.json';
import enSaves from './locales/en/saves.json';
import enSettings from './locales/en/settings.json';

declare module 'react-i18next' {
  interface CustomTypeOptions {
    defaultNS: 'common';
    resources: {
      common: typeof enCommon;
      chat: typeof enChat;
      settings: typeof enSettings;
      errors: typeof enErrors;
      onboarding: typeof enOnboarding;
      saves: typeof enSaves;
      character: typeof enCharacter;
    };
  }
}
