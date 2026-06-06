import { LazyStore } from '@tauri-apps/plugin-store';
import { beforeEach, describe, expect, it } from 'vitest';
import { persistStorage } from '../persistStorage';
import type { ApiKey, BaseUrl, OpenaiCompatConfig } from '../providers';
import type { SettingsData } from '../settings';
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
    settings.delete('discovered_catalogs'),
    settings.delete('image_enabled'),
    settings.delete('image_preset'),
    settings.delete('image_style_lora'),
    settings.delete('video_enabled'),
    settings.delete('video_mode'),
    settings.delete('vision_enabled'),
    settings.delete('reasoning_enabled'),
    settings.delete('reasoning_budget'),
    settings.delete('license_restricted_mode'),
    settings.delete('agent_max_rounds'),
  ]);
}

describe('persistStorage', () => {
  beforeEach(async () => {
    await clearStores();
  });

  it('returns null when nothing is on disk', async () => {
    expect(await persistStorage.getItem('any')).toBeNull();
  });

  it('parses a legacy anthropic-persisted blob without throwing and strips the anthropic config', async () => {
    // M11 Batch D.5: native Anthropic was removed. A legacy settings.json with
    // active_provider='anthropic' + a providers.anthropic blob must still PARSE
    // (the picklist keeps 'anthropic' for tolerance), with the stale anthropic
    // provider config silently stripped by ProvidersMapSchema. The store-side
    // reset to openai-compat happens later in the rehydration merge.
    await strongholdSecretsStore.set('providers', {
      anthropic: { kind: 'anthropic', apiKey: 'sk-ant-legacy', model: 'claude-haiku' },
      'openai-compat': null,
      'local-mistralrs': null,
    });
    await settings.set('active_provider', 'anthropic');

    const parsed = await persistStorage.getItem('any');
    expect(parsed).not.toBeNull();
    // raw value preserved at the storage boundary; reset is the store's job
    expect(parsed?.state.settings?.activeProvider).toBe('anthropic');
    expect(parsed?.state.settings?.providers).not.toHaveProperty('anthropic');
  });

  it('writes provider configs to the Stronghold vault and prefs to settings.json', async () => {
    const cfg: OpenaiCompatConfig = {
      kind: 'openai-compat',
      baseUrl: 'https://openrouter.ai/api/v1' as BaseUrl,
      apiKey: 'sk-or-real' as ApiKey,
      model: 'anthropic/claude-3.5-sonnet',
    };

    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'openai-compat',
          providers: {
            'openai-compat': cfg,
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
    expect(await strongholdSecretsStore.get('providers')).toMatchObject({ 'openai-compat': cfg });
    expect(await legacySecrets.get('providers')).toBeUndefined();

    // Prefs live in settings.json under the legacy snake_case keys.
    expect(await settings.get('active_provider')).toBe('openai-compat');
    expect(await settings.get('ui_language')).toBe('ru');
    expect(await settings.get('narration_language')).toBe('en');
  });

  it('one-shot migrates legacy plaintext secrets.json into the vault on first read', async () => {
    const cfg: OpenaiCompatConfig = {
      kind: 'openai-compat',
      baseUrl: 'https://openrouter.ai/api/v1' as BaseUrl,
      apiKey: 'sk-or-legacy' as ApiKey,
      model: 'anthropic/claude-3.5-sonnet',
    };
    // Seed only the legacy plaintext store - simulates an upgrade from M4.5.
    await legacySecrets.set('providers', {
      'openai-compat': cfg,
      'local-mistralrs': null,
    });

    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.settings?.providers).toMatchObject({ 'openai-compat': cfg });

    // The legacy entry was drained into the vault and removed from the
    // plaintext file as part of the migration.
    expect(await strongholdSecretsStore.get('providers')).toMatchObject({ 'openai-compat': cfg });
    expect(await legacySecrets.get('providers')).toBeUndefined();
  });

  it('round-trips a full snapshot back through getItem', async () => {
    const cfg: OpenaiCompatConfig = {
      kind: 'openai-compat',
      baseUrl: 'https://openrouter.ai/api/v1' as BaseUrl,
      apiKey: 'sk-or-roundtrip' as ApiKey,
      model: 'anthropic/claude-3-opus',
    };

    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'openai-compat',
          providers: {
            'openai-compat': cfg,
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
      activeProvider: 'openai-compat',
      providers: {
        'openai-compat': cfg,
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

  it('writes discoveredCatalogs to plaintext settings.json (NOT Stronghold)', async () => {
    const cat = {
      cacheKey: 'h',
      cachedAt: '2026-05-17T12:00:00Z',
      source: 'curated' as const,
      models: [
        {
          model_id: 'claude-opus-4-7',
          display_name: 'Claude Opus 4.7',
          capabilities: {
            vision_input: true,
            reasoning: true,
            tool_calls: true,
            streaming: true,
          },
          source: 'curated' as const,
          context_length: 1_000_000,
        },
      ],
    };
    await persistStorage.setItem('any', {
      state: {
        settings: { discoveredCatalogs: { 'openai-compat': cat } },
      },
      version: 0,
    });
    expect(await settings.get('discovered_catalogs')).toEqual({ 'openai-compat': cat });
    // Must NOT have leaked into Stronghold.
    expect(await strongholdSecretsStore.get('discovered_catalogs')).toBeUndefined();
  });

  it('round-trips discoveredCatalogs through getItem', async () => {
    const cat = {
      cacheKey: 'h',
      cachedAt: '2026-05-17T12:00:00Z',
      source: 'discovered-api' as const,
      models: [],
      next_cursor: null,
    };
    await persistStorage.setItem('any', {
      state: {
        settings: { discoveredCatalogs: { 'openai-compat': cat } },
      },
      version: 0,
    });
    const loaded = await persistStorage.getItem('any');
    expect(loaded?.state.settings?.discoveredCatalogs?.['openai-compat']).toEqual(cat);
  });

  it('omits discoveredCatalogs from load when nothing is on disk', async () => {
    // Just write any unrelated bit so getItem returns non-null.
    await persistStorage.setItem('any', {
      state: { settings: { uiLanguage: 'en' } },
      version: 0,
    });
    const loaded = await persistStorage.getItem('any');
    const dc = loaded?.state.settings?.discoveredCatalogs;
    expect(dc === undefined || JSON.stringify(dc) === '{}').toBe(true);
  });

  it('removeItem also clears discovered_catalogs from settings.json', async () => {
    await settings.set('discovered_catalogs', { anthropic: { cacheKey: 'h', models: [] } });
    await persistStorage.removeItem('any');
    expect(await settings.get('discovered_catalogs')).toBeUndefined();
  });

  // M7-DM field round-trip coverage
  const M7_DM_FIELDS = [
    { key: 'imageEnabled', value: true },
    { key: 'imageEnabled', value: false },
    { key: 'imagePreset', value: 'fast' as const },
    { key: 'imagePreset', value: 'balanced' as const },
    { key: 'imagePreset', value: 'quality' as const },
    { key: 'imagePreset', value: 'quality-oss' as const },
    { key: 'imagePreset', value: 'cloud' as const },
    { key: 'imageStyleLora', value: 'cinematic' },
    { key: 'imageStyleLora', value: null },
    { key: 'videoEnabled', value: true },
    { key: 'videoEnabled', value: false },
    { key: 'videoMode', value: 'prerecorded' as const },
    { key: 'videoMode', value: 'live' as const },
    { key: 'videoMode', value: 'race' as const },
    { key: 'visionEnabled', value: true },
    { key: 'visionEnabled', value: false },
    { key: 'reasoningEnabled', value: true },
    { key: 'reasoningEnabled', value: false },
    { key: 'reasoningBudget', value: 'low' as const },
    { key: 'reasoningBudget', value: 'medium' as const },
    { key: 'reasoningBudget', value: 'high' as const },
    { key: 'licenseRestrictedMode', value: true },
    { key: 'licenseRestrictedMode', value: false },
    { key: 'sceneTransitionsEnabled', value: false },
    { key: 'sceneTransitionsEnabled', value: true },
    { key: 'agentMaxRounds', value: 1 },
    { key: 'agentMaxRounds', value: 12 },
    { key: 'agentMaxRounds', value: 50 },
  ] as const;

  it.each(M7_DM_FIELDS)('persists settings.$key = $value across round-trip', async ({
    key,
    value,
  }) => {
    await persistStorage.setItem('test', {
      state: { settings: { [key]: value } as Partial<SettingsData> },
      version: 0,
    });
    const loaded = await persistStorage.getItem('test');
    expect(loaded?.state.settings?.[key]).toEqual(value);
  });

  it('removeItem clears all M7-DM fields from settings.json', async () => {
    await persistStorage.setItem('any', {
      state: {
        settings: {
          imageEnabled: true,
          imagePreset: 'quality',
          imageStyleLora: 'cinematic',
          videoEnabled: true,
          videoMode: 'live',
          visionEnabled: true,
          reasoningEnabled: true,
          reasoningBudget: 'high',
          licenseRestrictedMode: true,
          agentMaxRounds: 10,
        },
      },
      version: 0,
    });
    await persistStorage.removeItem('any');
    expect(await settings.get('image_enabled')).toBeUndefined();
    expect(await settings.get('image_preset')).toBeUndefined();
    expect(await settings.get('image_style_lora')).toBeUndefined();
    expect(await settings.get('video_enabled')).toBeUndefined();
    expect(await settings.get('video_mode')).toBeUndefined();
    expect(await settings.get('vision_enabled')).toBeUndefined();
    expect(await settings.get('reasoning_enabled')).toBeUndefined();
    expect(await settings.get('reasoning_budget')).toBeUndefined();
    expect(await settings.get('license_restricted_mode')).toBeUndefined();
    expect(await settings.get('agent_max_rounds')).toBeUndefined();
  });

  it('removeItem clears the vault, the legacy file, and the prefs file', async () => {
    await persistStorage.setItem('any', {
      state: {
        settings: {
          activeProvider: 'openai-compat',
          providers: {
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
