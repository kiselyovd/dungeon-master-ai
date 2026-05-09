import { LazyStore } from '@tauri-apps/plugin-store';
import { beforeEach, describe, expect, it } from 'vitest';
import { persistStorage } from '../persistStorage';
import type { AnthropicConfig, ApiKey } from '../providers';
import { strongholdSecretsStore } from '../strongholdSecretsStore';

/**
 * persistStorage is the bridge between Zustand's persist middleware and the
 * two on-disk surfaces (mocked here as in-memory buckets):
 *
 * - encrypted Stronghold vault (`strongholdSecretsStore`) for credentials
 * - plaintext `LazyStore` `settings.json` for non-sensitive prefs
 * - legacy `LazyStore` `secrets.json` for one-shot migration on first read
 *
 * These tests lock down the split, the on-disk key naming, the round-trip
 * semantics, and the legacy-to-vault migration path.
 */

const legacySecrets = new LazyStore('secrets.json');
const settings = new LazyStore('settings.json');

async function clearStores() {
  strongholdSecretsStore._resetForTests();
  await Promise.all([
    strongholdSecretsStore.delete('providers'),
    strongholdSecretsStore.delete('replicate_api_key'),
    legacySecrets.delete('providers'),
    legacySecrets.delete('replicate_api_key'),
    settings.delete('active_provider'),
    settings.delete('ui_language'),
    settings.delete('narration_language'),
    settings.delete('system_prompt'),
    settings.delete('temperature'),
    settings.delete('chat_panel_width'),
    settings.delete('active_campaign_id'),
    settings.delete('active_session_id'),
    settings.delete('current_scene'),
    settings.delete('onboarding_completed'),
    settings.delete('hero_class'),
  ]);
}

describe('persistStorage', () => {
  beforeEach(async () => {
    await clearStores();
  });

  it('returns null when nothing is on disk', async () => {
    expect(await persistStorage.getItem('any')).toBeNull();
  });

  it('writes provider configs to the Stronghold vault and prefs to settings.json', async () => {
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

    // Provider blob lives in the encrypted Stronghold vault, NOT in the
    // legacy plaintext secrets.json.
    expect(await strongholdSecretsStore.get('providers')).toMatchObject({ anthropic: cfg });
    expect(await legacySecrets.get('providers')).toBeUndefined();

    // Prefs live in settings.json under the legacy snake_case keys.
    expect(await settings.get('active_provider')).toBe('anthropic');
    expect(await settings.get('ui_language')).toBe('ru');
    expect(await settings.get('narration_language')).toBe('en');
  });

  it('one-shot migrates legacy plaintext secrets.json into the vault on first read', async () => {
    const cfg: AnthropicConfig = {
      kind: 'anthropic',
      apiKey: 'sk-ant-legacy' as ApiKey,
      model: 'claude-haiku',
    };
    // Seed only the legacy plaintext store - simulates an upgrade from M4.5.
    await legacySecrets.set('providers', {
      anthropic: cfg,
      'openai-compat': null,
      'local-mistralrs': null,
    });

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.settings?.providers).toMatchObject({ anthropic: cfg });

    // The legacy entry was drained into the vault and removed from the
    // plaintext file as part of the migration.
    expect(await strongholdSecretsStore.get('providers')).toMatchObject({ anthropic: cfg });
    expect(await legacySecrets.get('providers')).toBeUndefined();
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

  it('round-trips the current scene snapshot through settings.json', async () => {
    await persistStorage.setItem('any', {
      state: {
        session: {
          activeCampaignId: 'camp-1',
          activeSessionId: 'sess-1',
          loadError: null,
          currentScene: { name: 'Crimson Sanctuary', stepCounter: 7 },
        },
      },
      version: 0,
    });

    expect(await settings.get('current_scene')).toEqual({
      name: 'Crimson Sanctuary',
      stepCounter: 7,
    });

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.session?.currentScene).toEqual({
      name: 'Crimson Sanctuary',
      stepCounter: 7,
    });
  });

  it('persists a null currentScene as an explicit clear', async () => {
    // Seed first, then write null - the persisted key should reflect the
    // cleared scene rather than retaining the previous value.
    await settings.set('current_scene', { name: 'Stale Scene', stepCounter: 3 });
    await persistStorage.setItem('any', {
      state: {
        session: {
          activeCampaignId: null,
          activeSessionId: null,
          loadError: null,
          currentScene: null,
        },
      },
      version: 0,
    });
    expect(await settings.get('current_scene')).toBeNull();

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.session?.currentScene).toBeNull();
  });

  it('round-trips the onboarding-completed flag and chosen hero class', async () => {
    await persistStorage.setItem('any', {
      state: {
        onboarding: { completed: true },
        pc: { heroClass: 'wizard' },
      },
      version: 0,
    });

    expect(await settings.get('onboarding_completed')).toBe(true);
    expect(await settings.get('hero_class')).toBe('wizard');

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.onboarding?.completed).toBe(true);
    expect(loaded?.state.pc?.heroClass).toBe('wizard');
  });

  it('persists a null heroClass as an explicit clear', async () => {
    await settings.set('hero_class', 'fighter');
    await persistStorage.setItem('any', {
      state: {
        pc: { heroClass: null },
      },
      version: 0,
    });
    expect(await settings.get('hero_class')).toBeNull();

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.pc?.heroClass).toBeNull();
  });

  it('removeItem clears the vault, the legacy file, and the prefs file', async () => {
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

    expect(await strongholdSecretsStore.get('providers')).toBeUndefined();
    expect(await legacySecrets.get('providers')).toBeUndefined();
    expect(await settings.get('active_provider')).toBeUndefined();
    expect(await settings.get('ui_language')).toBeUndefined();
    expect(await settings.get('narration_language')).toBeUndefined();
  });
});
