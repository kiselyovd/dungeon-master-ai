/**
 * PersistStorage adapter for Zustand's persist middleware, backed by
 * tauri-plugin-store. Splits the persisted Settings slice across two on-disk
 * files:
 *
 * - `secrets.json` keeps the per-provider configs (api keys, base URLs).
 * - `settings.json` keeps the non-sensitive prefs (active provider, languages).
 *
 * The split exists so the M2 keychain swap touches only `secrets.json`.
 *
 * The on-disk key layout intentionally matches the original `loadAll/save*`
 * helpers (`providers`, `active_provider`, `ui_language`, `narration_language`)
 * so users upgrading from M1.5 keep their saved settings.
 */
import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';
import type { PersistStorage, StorageValue } from 'zustand/middleware';
import {
  AnthropicConfigSchema,
  LocalMistralRsConfigSchema,
  OpenaiCompatConfigSchema,
} from './providers';
import type { Language, ProvidersMap, SettingsData } from './settings';

const SECRETS_FILE = 'secrets.json';
const SETTINGS_FILE = 'settings.json';

const KEY_PROVIDERS = 'providers';
const KEY_ACTIVE_PROVIDER = 'active_provider';
const KEY_UI_LANGUAGE = 'ui_language';
const KEY_NARRATION_LANGUAGE = 'narration_language';

const secretsStore = new LazyStore(SECRETS_FILE);
const settingsStore = new LazyStore(SETTINGS_FILE);

const ProviderKindSchema = v.picklist(['anthropic', 'openai-compat', 'local-mistralrs']);
const LanguageSchema = v.picklist(['en', 'ru']);

const ProvidersMapSchema = v.object({
  anthropic: v.nullable(AnthropicConfigSchema),
  'openai-compat': v.nullable(OpenaiCompatConfigSchema),
  'local-mistralrs': v.nullable(LocalMistralRsConfigSchema),
});

export interface PersistedSettings {
  settings: Partial<SettingsData>;
}

export const persistStorage: PersistStorage<PersistedSettings> = {
  async getItem(_name): Promise<StorageValue<PersistedSettings> | null> {
    const [providersRaw, activeRaw, uiRaw, narrRaw] = await Promise.all([
      secretsStore.get(KEY_PROVIDERS),
      settingsStore.get(KEY_ACTIVE_PROVIDER),
      settingsStore.get(KEY_UI_LANGUAGE),
      settingsStore.get(KEY_NARRATION_LANGUAGE),
    ]);

    const providersParsed = v.safeParse(ProvidersMapSchema, providersRaw);
    const activeParsed = v.safeParse(ProviderKindSchema, activeRaw);
    const uiParsed = v.safeParse(LanguageSchema, uiRaw);
    const narrParsed = v.safeParse(LanguageSchema, narrRaw);

    if (
      !providersParsed.success &&
      !activeParsed.success &&
      !uiParsed.success &&
      !narrParsed.success
    ) {
      return null;
    }

    const settings: Partial<SettingsData> = {};
    if (providersParsed.success) settings.providers = providersParsed.output as ProvidersMap;
    if (activeParsed.success) settings.activeProvider = activeParsed.output;
    if (uiParsed.success) settings.uiLanguage = uiParsed.output as Language;
    if (narrParsed.success) settings.narrationLanguage = narrParsed.output as Language;

    return { state: { settings }, version: 0 };
  },

  async setItem(_name, value): Promise<void> {
    const settings = value.state.settings ?? {};
    const writes: Promise<unknown>[] = [];
    if (settings.providers !== undefined) {
      writes.push(secretsStore.set(KEY_PROVIDERS, settings.providers));
    }
    if (settings.activeProvider !== undefined) {
      writes.push(settingsStore.set(KEY_ACTIVE_PROVIDER, settings.activeProvider));
    }
    if (settings.uiLanguage !== undefined) {
      writes.push(settingsStore.set(KEY_UI_LANGUAGE, settings.uiLanguage));
    }
    if (settings.narrationLanguage !== undefined) {
      writes.push(settingsStore.set(KEY_NARRATION_LANGUAGE, settings.narrationLanguage));
    }
    await Promise.all(writes);
    await Promise.all([secretsStore.save(), settingsStore.save()]);
  },

  async removeItem(_name): Promise<void> {
    await Promise.all([
      secretsStore.delete(KEY_PROVIDERS),
      settingsStore.delete(KEY_ACTIVE_PROVIDER),
      settingsStore.delete(KEY_UI_LANGUAGE),
      settingsStore.delete(KEY_NARRATION_LANGUAGE),
    ]);
    await Promise.all([secretsStore.save(), settingsStore.save()]);
  },
};
