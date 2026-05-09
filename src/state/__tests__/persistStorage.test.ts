import { LazyStore } from '@tauri-apps/plugin-store';
import { beforeEach, describe, expect, it } from 'vitest';
import { persistStorage } from '../persistStorage';
import type { AnthropicConfig, ApiKey } from '../providers';

/**
 * persistStorage is the bridge between Zustand's persist middleware and the
 * Tauri plugin-store. Two on-disk files (mocked here as in-memory buckets):
 * `secrets.json` for credentials, `settings.json` for prefs. These tests
 * lock down the split, the on-disk key naming, and the round-trip semantics.
 */

const secrets = new LazyStore('secrets.json');
const settings = new LazyStore('settings.json');

async function clearStores() {
  await Promise.all([
    secrets.delete('providers'),
    settings.delete('active_provider'),
    settings.delete('ui_language'),
    settings.delete('narration_language'),
  ]);
}

describe('persistStorage', () => {
  beforeEach(async () => {
    await clearStores();
  });

  it('returns null when nothing is on disk', async () => {
    expect(await persistStorage.getItem('any')).toBeNull();
  });

  it('writes provider configs to secrets.json and prefs to settings.json', async () => {
    const cfg: AnthropicConfig = {
      kind: 'anthropic',
      apiKey: 'sk-ant-real' as ApiKey,
      model: 'claude-haiku',
    };

    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'anthropic',
          providers: {
            anthropic: cfg,
            'openai-compat': null,
            'local-mistralrs': null,
          },
          uiLanguage: 'ru',
          narrationLanguage: 'en',
        },
      },
      version: 0,
    });

    // Provider blob lives in secrets.json.
    const providers = await secrets.get('providers');
    expect(providers).toMatchObject({ anthropic: cfg });

    // Prefs live in settings.json under the legacy snake_case keys.
    expect(await settings.get('active_provider')).toBe('anthropic');
    expect(await settings.get('ui_language')).toBe('ru');
    expect(await settings.get('narration_language')).toBe('en');
  });

  it('round-trips a full snapshot back through getItem', async () => {
    const cfg: AnthropicConfig = {
      kind: 'anthropic',
      apiKey: 'sk-ant-roundtrip' as ApiKey,
      model: 'claude-opus',
    };

    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'anthropic',
          providers: {
            anthropic: cfg,
            'openai-compat': null,
            'local-mistralrs': null,
          },
          uiLanguage: 'en',
          narrationLanguage: 'ru',
        },
      },
      version: 0,
    });

    const loaded = await persistStorage.getItem('any');
    expect(loaded).not.toBeNull();
    expect(loaded?.state.settings).toEqual({
      activeProvider: 'anthropic',
      providers: {
        anthropic: cfg,
        'openai-compat': null,
        'local-mistralrs': null,
      },
      uiLanguage: 'en',
      narrationLanguage: 'ru',
    });
  });

  it('drops malformed pieces individually rather than poisoning the whole load', async () => {
    // Garbage active_provider (not in the picklist) plus a valid uiLanguage.
    await settings.set('active_provider', 'NOT_A_PROVIDER');
    await settings.set('ui_language', 'ru');

    const loaded = await persistStorage.getItem('any');
    expect(loaded).not.toBeNull();
    expect(loaded?.state.settings?.activeProvider).toBeUndefined();
    expect(loaded?.state.settings?.uiLanguage).toBe('ru');
  });

  it('removeItem clears both files', async () => {
    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'anthropic',
          providers: {
            anthropic: null,
            'openai-compat': null,
            'local-mistralrs': null,
          },
          uiLanguage: 'en',
          narrationLanguage: 'en',
        },
      },
      version: 0,
    });

    await persistStorage.removeItem('any');

    expect(await secrets.get('providers')).toBeUndefined();
    expect(await settings.get('active_provider')).toBeUndefined();
    expect(await settings.get('ui_language')).toBeUndefined();
    expect(await settings.get('narration_language')).toBeUndefined();
  });
});
