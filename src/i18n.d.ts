/**
 * Type-augment react-i18next so `t('foo:bar')` is autocompleted and typo-checked.
 * The English bundle is the source of truth - if RU drifts, type-check is fine
 * (resources are merged), but missing keys are caught at compile time wherever
 * the typed `t` is consumed.
 */

import 'react-i18next';

import enAgent from './locales/en/agent.json';
import enCharacter from './locales/en/character.json';
import enChat from './locales/en/chat.json';
import enCombat from './locales/en/combat.json';
import enCommon from './locales/en/common.json';
import enErrors from './locales/en/errors.json';
import enLocalMode from './locales/en/local_mode.json';
import enOnboarding from './locales/en/onboarding.json';
import enSaves from './locales/en/saves.json';
import enSettings from './locales/en/settings.json';
import enWizard from './locales/en/wizard.json';

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
      combat: typeof enCombat;
      agent: typeof enAgent;
      local_mode: typeof enLocalMode;
      wizard: typeof enWizard;
    };
  }
}
