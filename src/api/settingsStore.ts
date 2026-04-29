/**
 * Persistence layer for the user-configured Settings.
 *
 * Splits storage into two LazyStore files:
 * - `secrets.json` keeps the per-provider configs (api keys, base URLs).
 *   These contain credentials, so we keep them isolated from prefs.
 * - `settings.json` keeps the non-sensitive prefs (active provider,
 *   languages).
 *
 * For now the storage is plain JSON inside the Tauri app data dir; the M2
 * roadmap calls for a swap to an OS-keychain-backed plugin (DPAPI on
 * Windows, Keychain on macOS, Secret Service on Linux). Consumer code
 * does not need to change - only this module.
 */

import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';
import {
  AnthropicConfigSchema,
  LocalMistralRsConfigSchema,
  OpenaiCompatConfigSchema,
} from '../state/providers';
import type { HydrateInput, Language, ProvidersMap } from '../state/settings';

const SECRETS_FILE = 'secrets.json';
const SETTINGS_FILE = 'settings.json';

const secretsStore = new LazyStore(SECRETS_FILE);
const settingsStore = new LazyStore(SETTINGS_FILE);

const KEY_PROVIDERS = 'providers';
const KEY_ACTIVE_PROVIDER = 'active_provider';
const KEY_UI_LANGUAGE = 'ui_language';
const KEY_NARRATION_LANGUAGE = 'narration_language';

const ProviderKindSchema = v.picklist(['anthropic', 'openai-compat', 'local-mistralrs']);
const LanguageSchema = v.picklist(['en', 'ru']);

const ProvidersMapSchema = v.object({
  anthropic: v.nullable(AnthropicConfigSchema),
  'openai-compat': v.nullable(OpenaiCompatConfigSchema),
  'local-mistralrs': v.nullable(LocalMistralRsConfigSchema),
});

export async function loadAll(): Promise<HydrateInput> {
  const [providersRaw, active, ui, narr] = await Promise.all([
    secretsStore.get(KEY_PROVIDERS),
    settingsStore.get(KEY_ACTIVE_PROVIDER),
    settingsStore.get(KEY_UI_LANGUAGE),
    settingsStore.get(KEY_NARRATION_LANGUAGE),
  ]);

  const out: HydrateInput = {};
  const providersParsed = v.safeParse(ProvidersMapSchema, providersRaw);
  if (providersParsed.success) {
    out.providers = providersParsed.output as ProvidersMap;
  }
  const activeParsed = v.safeParse(ProviderKindSchema, active);
  if (activeParsed.success) out.activeProvider = activeParsed.output;
  const uiParsed = v.safeParse(LanguageSchema, ui);
  if (uiParsed.success) out.uiLanguage = uiParsed.output as Language;
  const narrParsed = v.safeParse(LanguageSchema, narr);
  if (narrParsed.success) out.narrationLanguage = narrParsed.output as Language;

  return out;
}

export async function saveProviders(providers: ProvidersMap): Promise<void> {
  await secretsStore.set(KEY_PROVIDERS, providers);
  await secretsStore.save();
}

export async function saveActiveProvider(kind: HydrateInput['activeProvider']): Promise<void> {
  if (kind === undefined) return;
  await settingsStore.set(KEY_ACTIVE_PROVIDER, kind);
  await settingsStore.save();
}

export async function saveUiLanguage(lang: Language): Promise<void> {
  await settingsStore.set(KEY_UI_LANGUAGE, lang);
  await settingsStore.save();
}

export async function saveNarrationLanguage(lang: Language): Promise<void> {
  await settingsStore.set(KEY_NARRATION_LANGUAGE, lang);
  await settingsStore.save();
}
