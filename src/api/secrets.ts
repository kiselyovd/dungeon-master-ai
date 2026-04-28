import { LazyStore } from '@tauri-apps/plugin-store';

const SECRETS_FILE = 'secrets.json';
const SETTINGS_FILE = 'settings.json';

const secrets = new LazyStore(SECRETS_FILE);
const settings = new LazyStore(SETTINGS_FILE);

export async function getAnthropicApiKey(): Promise<string | undefined> {
  const v = await secrets.get<string>('anthropic_api_key');
  return v ?? undefined;
}

export async function setAnthropicApiKey(key: string | undefined): Promise<void> {
  if (key === undefined || key === '') {
    await secrets.delete('anthropic_api_key');
  } else {
    await secrets.set('anthropic_api_key', key);
  }
  await secrets.save();
}

export async function getUiLanguage(): Promise<'en' | 'ru' | undefined> {
  const v = await settings.get<'en' | 'ru'>('ui_language');
  return v ?? undefined;
}

export async function setUiLanguage(lang: 'en' | 'ru'): Promise<void> {
  await settings.set('ui_language', lang);
  await settings.save();
}

export async function getNarrationLanguage(): Promise<'en' | 'ru' | undefined> {
  const v = await settings.get<'en' | 'ru'>('narration_language');
  return v ?? undefined;
}

export async function setNarrationLanguage(lang: 'en' | 'ru'): Promise<void> {
  await settings.set('narration_language', lang);
  await settings.save();
}
