import { describe, expect, it } from 'vitest';
import { create } from 'zustand';
import type { DiscoveredCatalog } from '../discoveredCatalogs';
import {
  applyProviderMigration,
  createSettingsSlice,
  DEFAULT_CHAT_WIDTH,
  MAX_CHAT_WIDTH,
  MIN_CHAT_WIDTH,
  type SettingsData,
  type SettingsSlice,
} from '../settings';

function freshSettingsStore() {
  return create<SettingsSlice>()((...a) => ({
    ...createSettingsSlice(...a),
  }));
}

describe('SettingsSlice model tab fields', () => {
  it('has default systemPrompt as empty string', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.systemPrompt).toBe('');
  });

  it('has default temperature of 0.7', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.temperature).toBe(0.7);
  });

  it('has null replicateApiKey by default', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.replicateApiKey).toBeNull();
  });

  it('setSystemPrompt updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setSystemPrompt('You are a DM.');
    expect(store.getState().settings.systemPrompt).toBe('You are a DM.');
  });

  it('setTemperature updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setTemperature(1.2);
    expect(store.getState().settings.temperature).toBe(1.2);
  });

  it('setReplicateApiKey updates value', () => {
    const store = freshSettingsStore();
    store.getState().settings.setReplicateApiKey('r8_xyz');
    expect(store.getState().settings.replicateApiKey).toBe('r8_xyz');
  });
});

describe('SettingsSlice chat panel width', () => {
  it('exports MIN/MAX bounds at 360 and 640', () => {
    expect(MIN_CHAT_WIDTH).toBe(360);
    expect(MAX_CHAT_WIDTH).toBe(640);
  });

  it('defaults chatPanelWidth to 480', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.chatPanelWidth).toBe(DEFAULT_CHAT_WIDTH);
    expect(DEFAULT_CHAT_WIDTH).toBe(480);
  });

  it('setChatPanelWidth stores a value within range as-is', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(420);
    expect(store.getState().settings.chatPanelWidth).toBe(420);
  });

  it('setChatPanelWidth clamps values below MIN_CHAT_WIDTH up to the floor', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(200);
    expect(store.getState().settings.chatPanelWidth).toBe(MIN_CHAT_WIDTH);
  });

  it('setChatPanelWidth clamps values above MAX_CHAT_WIDTH down to the ceiling', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(900);
    expect(store.getState().settings.chatPanelWidth).toBe(MAX_CHAT_WIDTH);
  });

  it('setChatPanelWidth falls back to default for non-finite input', () => {
    const store = freshSettingsStore();
    store.getState().settings.setChatPanelWidth(Number.NaN);
    expect(store.getState().settings.chatPanelWidth).toBe(DEFAULT_CHAT_WIDTH);
  });
});

describe('SettingsSlice discoveredCatalogs', () => {
  const fakeCat = (overrides: Partial<DiscoveredCatalog> = {}): DiscoveredCatalog => ({
    cacheKey: 'k',
    cachedAt: '2026-05-17T12:00:00Z',
    source: 'curated',
    models: [],
    ...overrides,
  });

  it('defaults discoveredCatalogs to an empty object', () => {
    const store = freshSettingsStore();
    expect(store.getState().settings.discoveredCatalogs).toEqual({});
  });

  it('setDiscoveredCatalog stores a catalog under provider key', () => {
    const store = freshSettingsStore();
    const cat = fakeCat({ cacheKey: 'h1' });
    store.getState().settings.setDiscoveredCatalog('openai-compat', cat);
    expect(store.getState().settings.discoveredCatalogs['openai-compat']).toBe(cat);
  });

  it('setDiscoveredCatalog overwrites an existing entry for the same provider', () => {
    const store = freshSettingsStore();
    const first = fakeCat({ cacheKey: 'a' });
    const second = fakeCat({ cacheKey: 'b', source: 'discovered-api' });
    store.getState().settings.setDiscoveredCatalog('openai-compat', first);
    store.getState().settings.setDiscoveredCatalog('openai-compat', second);
    expect(store.getState().settings.discoveredCatalogs['openai-compat']).toBe(second);
  });

  it('setDiscoveredCatalog for one provider leaves others untouched', () => {
    const store = freshSettingsStore();
    const a = fakeCat({ cacheKey: 'a' });
    const b = fakeCat({ cacheKey: 'b', source: 'discovered-api' });
    store.getState().settings.setDiscoveredCatalog('local-mistralrs', a);
    store.getState().settings.setDiscoveredCatalog('openai-compat', b);
    expect(store.getState().settings.discoveredCatalogs['local-mistralrs']).toBe(a);
    expect(store.getState().settings.discoveredCatalogs['openai-compat']).toBe(b);
  });

  it('clearDiscoveredCatalog sets the entry to null', () => {
    const store = freshSettingsStore();
    store.getState().settings.setDiscoveredCatalog('openai-compat', fakeCat());
    store.getState().settings.clearDiscoveredCatalog('openai-compat');
    expect(store.getState().settings.discoveredCatalogs['openai-compat']).toBeNull();
  });

  it('invalidateProviderCatalog clears the entry too', () => {
    const store = freshSettingsStore();
    store.getState().settings.setDiscoveredCatalog('openai-compat', fakeCat());
    store.getState().settings.invalidateProviderCatalog('openai-compat');
    expect(store.getState().settings.discoveredCatalogs['openai-compat']).toBeNull();
  });
});

describe('applyProviderMigration', () => {
  it('resets a legacy anthropic activeProvider to openai-compat and raises the notice', () => {
    const input: Partial<SettingsData> = {
      activeProvider: 'anthropic' as unknown as SettingsData['activeProvider'],
      providers: {
        // legacy blob - applyProviderMigration must strip this key
        anthropic: { kind: 'anthropic', apiKey: 'sk-ant', model: 'claude' },
        'openai-compat': null,
        'local-mistralrs': null,
      } as unknown as SettingsData['providers'],
    };
    const out = applyProviderMigration(input);
    expect(out.activeProvider).toBe('openai-compat');
    expect(out.providerMigrationNotice).toBe(true);
    expect((out.providers as unknown as Record<string, unknown>).anthropic).toBeUndefined();
  });

  it('leaves a non-anthropic activeProvider unchanged and does not raise the notice', () => {
    const input: Partial<SettingsData> = {
      activeProvider: 'local-mistralrs',
      providers: {
        'openai-compat': null,
        'local-mistralrs': null,
      },
    };
    const out = applyProviderMigration(input);
    expect(out.activeProvider).toBe('local-mistralrs');
    expect(out.providerMigrationNotice).toBeFalsy();
  });
});
