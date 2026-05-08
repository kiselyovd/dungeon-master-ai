/**
 * PersistStorage adapter for Zustand's persist middleware, backed by
 * tauri-plugin-store. Splits the persisted Settings slice across two on-disk
 * files:
 *
 * - `secrets.json` keeps the per-provider configs (api keys, base URLs) and
 *   the Replicate API key (M3).
 * - `settings.json` keeps the non-sensitive prefs (active provider, languages,
 *   system prompt, temperature).
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
const KEY_SYSTEM_PROMPT = 'system_prompt';
const KEY_TEMPERATURE = 'temperature';
const KEY_REPLICATE_API_KEY = 'replicate_api_key';

const secretsStore = new LazyStore(SECRETS_FILE);
const settingsStore = new LazyStore(SETTINGS_FILE);

const ProviderKindSchema = v.picklist(['anthropic', 'openai-compat', 'local-mistralrs']);
const LanguageSchema = v.picklist(['en', 'ru']);

const ProvidersMapSchema = v.object({
  anthropic: v.nullable(AnthropicConfigSchema),
  'openai-compat': v.nullable(OpenaiCompatConfigSchema),
  'local-mistralrs': v.nullable(LocalMistralRsConfigSchema),
});

const SystemPromptSchema = v.string();
const TemperatureSchema = v.pipe(v.number(), v.minValue(0), v.maxValue(2));
const ReplicateKeySchema = v.nullable(v.string());

export interface PersistedSettings {
  settings: Partial<SettingsData>;
}

export const persistStorage: PersistStorage<PersistedSettings> = {
  async getItem(_name): Promise<StorageValue<PersistedSettings> | null> {
    const [providersRaw, activeRaw, uiRaw, narrRaw, sysRaw, tempRaw, replicateRaw] =
      await Promise.all([
        secretsStore.get(KEY_PROVIDERS),
        settingsStore.get(KEY_ACTIVE_PROVIDER),
        settingsStore.get(KEY_UI_LANGUAGE),
        settingsStore.get(KEY_NARRATION_LANGUAGE),
        settingsStore.get(KEY_SYSTEM_PROMPT),
        settingsStore.get(KEY_TEMPERATURE),
        secretsStore.get(KEY_REPLICATE_API_KEY),
      ]);

    const providersParsed = v.safeParse(ProvidersMapSchema, providersRaw);
    const activeParsed = v.safeParse(ProviderKindSchema, activeRaw);
    const uiParsed = v.safeParse(LanguageSchema, uiRaw);
    const narrParsed = v.safeParse(LanguageSchema, narrRaw);
    const sysParsed = v.safeParse(SystemPromptSchema, sysRaw);
    const tempParsed = v.safeParse(TemperatureSchema, tempRaw);
    const replicateParsed = v.safeParse(ReplicateKeySchema, replicateRaw);

    if (
      !providersParsed.success &&
      !activeParsed.success &&
      !uiParsed.success &&
      !narrParsed.success &&
      !sysParsed.success &&
      !tempParsed.success &&
      !replicateParsed.success
    ) {
      return null;
    }

    const settings: Partial<SettingsData> = {};
    if (providersParsed.success) settings.providers = providersParsed.output as ProvidersMap;
    if (activeParsed.success) settings.activeProvider = activeParsed.output;
    if (uiParsed.success) settings.uiLanguage = uiParsed.output as Language;
    if (narrParsed.success) settings.narrationLanguage = narrParsed.output as Language;
    if (sysParsed.success) settings.systemPrompt = sysParsed.output;
    if (tempParsed.success) settings.temperature = tempParsed.output;
    if (replicateParsed.success) settings.replicateApiKey = replicateParsed.output;

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
    if (settings.systemPrompt !== undefined) {
      writes.push(settingsStore.set(KEY_SYSTEM_PROMPT, settings.systemPrompt));
    }
    if (settings.temperature !== undefined) {
      writes.push(settingsStore.set(KEY_TEMPERATURE, settings.temperature));
    }
    if (settings.replicateApiKey !== undefined) {
      writes.push(secretsStore.set(KEY_REPLICATE_API_KEY, settings.replicateApiKey));
    }
    await Promise.all(writes);
    await Promise.all([secretsStore.save(), settingsStore.save()]);
  },

  async removeItem(_name): Promise<void> {
    await Promise.all([
      secretsStore.delete(KEY_PROVIDERS),
      secretsStore.delete(KEY_REPLICATE_API_KEY),
      settingsStore.delete(KEY_ACTIVE_PROVIDER),
      settingsStore.delete(KEY_UI_LANGUAGE),
      settingsStore.delete(KEY_NARRATION_LANGUAGE),
      settingsStore.delete(KEY_SYSTEM_PROMPT),
      settingsStore.delete(KEY_TEMPERATURE),
    ]);
    await Promise.all([secretsStore.save(), settingsStore.save()]);
  },
};
