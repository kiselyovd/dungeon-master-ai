import { describe, expect, it } from 'vitest';
import { DEFAULTS_V2, migrateLegacySettings } from '../settingsMigration';

describe('migrateLegacySettings', () => {
  it('fresh install returns defaults without reset flag', () => {
    const { config, didReset } = migrateLegacySettings(null);
    expect(config).toEqual(DEFAULTS_V2);
    expect(didReset).toBe(false);
  });

  it('undefined input treated as fresh install', () => {
    const { config, didReset } = migrateLegacySettings(undefined);
    expect(config).toEqual(DEFAULTS_V2);
    expect(didReset).toBe(false);
  });

  it('legacy v1 anthropic maps to chat.activeProviderId=anthropic + persists model', () => {
    const v1 = {
      activeProvider: 'anthropic',
      providers: { anthropic: { apiKey: 'sk-test', model: 'claude-opus-4-7' } },
    };
    const { config, didReset } = migrateLegacySettings(v1);
    expect(config.chat.activeProviderId).toBe('anthropic');
    expect(config.chat.activeModelId).toBe('claude-opus-4-7');
    expect(didReset).toBe(false);
  });

  it('legacy v1 with replicateApiKey moves to image.providers.replicate', () => {
    const v1 = {
      activeProvider: 'anthropic',
      providers: { anthropic: { apiKey: 'sk', model: 'claude-haiku-4-5-20251001' } },
      replicateApiKey: 'r8_xxx',
    };
    const { config } = migrateLegacySettings(v1);
    expect(config.image.activeProviderId).toBe('replicate');
    expect(config.image.preset).toBe('cloud');
    expect((config.image.providers as { replicate: { api_key: string } }).replicate.api_key).toBe(
      'r8_xxx',
    );
  });

  it('legacy v1 local-mistralrs canonicalises modelPath qwen3_5_4b -> qwen3.5-4b', () => {
    const v1 = {
      activeProvider: 'local-mistralrs',
      providers: { 'local-mistralrs': { modelPath: 'qwen3_5_4b', contextWindow: 8192 } },
    };
    const { config } = migrateLegacySettings(v1);
    expect(config.chat.activeProviderId).toBe('local-mistralrs');
    expect(config.chat.activeModelId).toBe('qwen3.5-4b');
  });

  it('corrupt input (string) returns defaults with didReset=true', () => {
    const { config, didReset } = migrateLegacySettings('not-an-object');
    expect(config).toEqual(DEFAULTS_V2);
    expect(didReset).toBe(true);
  });

  it('corrupt input (number) returns defaults with didReset=true', () => {
    const { config, didReset } = migrateLegacySettings(42);
    expect(config).toEqual(DEFAULTS_V2);
    expect(didReset).toBe(true);
  });

  it('partial v2 (already migrated) passes through with defaults filled in', () => {
    const v2 = {
      chat: { activeProviderId: 'anthropic', activeModelId: 'claude-opus-4-7' },
    };
    const { config, didReset } = migrateLegacySettings(v2);
    expect(config.chat.activeProviderId).toBe('anthropic');
    expect(config.chat.activeModelId).toBe('claude-opus-4-7');
    expect(config.image.enabled).toBe(true);
    expect(config.image.preset).toBe('balanced');
    expect(config.video.enabled).toBe(false);
    expect(didReset).toBe(false);
  });

  it('full v2 round-trips unchanged', () => {
    const v2 = { ...DEFAULTS_V2 };
    const { config, didReset } = migrateLegacySettings(v2);
    expect(config).toEqual(DEFAULTS_V2);
    expect(didReset).toBe(false);
  });
});
