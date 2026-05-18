import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { SettingsData } from '../../state/settings';
import { useStore } from '../../state/useStore';
import { setBackendPortForTesting } from '../client';
import { postSettingsV2, toV2Wire } from '../settings';

// Read whatever DEFAULT_SETTINGS exists; if not exported, build a baseline inline.
function baseSettings(): SettingsData {
  return {
    activeProvider: 'anthropic',
    providers: {
      anthropic: {
        kind: 'anthropic',
        apiKey: 'sk-test' as never,
        model: 'claude-haiku-4-5-20251001',
      },
      'openai-compat': null,
      'local-mistralrs': null,
    },
    uiLanguage: 'en',
    narrationLanguage: 'en',
    systemPrompt: 'DM',
    temperature: 0.7,
    replicateApiKey: null,
    chatPanelWidth: 480,
    sceneTransitionsEnabled: true,
    imageEnabled: true,
    imagePreset: 'balanced',
    imageStyleLora: null,
    videoEnabled: false,
    videoMode: 'prerecorded',
    visionEnabled: false,
    reasoningEnabled: false,
    reasoningBudget: 'medium',
    licenseRestrictedMode: false,
    agentMaxRounds: 8,
    discoveredCatalogs: {},
  } as SettingsData;
}

describe('toV2Wire', () => {
  it('maps anthropic baseline correctly', () => {
    const wire = toV2Wire(baseSettings());
    expect(wire.chat.active_provider_id).toBe('anthropic');
    expect(wire.chat.active_model_id).toBe('claude-haiku-4-5-20251001');
    expect(wire.chat.providers).toEqual({ anthropic: { api_key: 'sk-test' } });
    expect(wire.image.active_provider_id).toBe('local-sdxl-lightning');
    expect(wire.image.active_model_id).toBe('sdxl-lightning-4step');
    expect(wire.behavior.scene_transitions).toBe('auto');
  });

  it.each([
    ['fast', 'local-sdxl-turbo', 'sdxl-turbo-fp16'],
    ['balanced', 'local-sdxl-lightning', 'sdxl-lightning-4step'],
    ['quality', 'local-nunchaku-flux', 'flux-dev-int4-turbo-alpha-8step'],
    ['quality-oss', 'local-z-image-turbo', 'z-image-turbo-svdq-int4'],
    ['cloud', 'replicate', 'stability-ai/sdxl'],
  ] as const)('maps preset %s to provider %s + model %s', (preset, providerId, modelId) => {
    const s = baseSettings();
    s.imagePreset = preset;
    const wire = toV2Wire(s);
    expect(wire.image.active_provider_id).toBe(providerId);
    expect(wire.image.active_model_id).toBe(modelId);
  });

  it('maps scene_transitions enum from boolean', () => {
    const on = baseSettings();
    on.sceneTransitionsEnabled = true;
    expect(toV2Wire(on).behavior.scene_transitions).toBe('auto');
    const off = baseSettings();
    off.sceneTransitionsEnabled = false;
    expect(toV2Wire(off).behavior.scene_transitions).toBe('off');
  });

  it('maps openai-compat provider with baseUrl + apiKey', () => {
    const s = baseSettings();
    s.activeProvider = 'openai-compat';
    s.providers['openai-compat'] = {
      kind: 'openai-compat',
      baseUrl: 'http://localhost:1234/v1' as never,
      apiKey: 'oc-key' as never,
      model: 'custom',
    };
    const wire = toV2Wire(s);
    expect(wire.chat.providers).toEqual({
      'openai-compat': { base_url: 'http://localhost:1234/v1', api_key: 'oc-key' },
    });
    expect(wire.chat.active_model_id).toBe('custom');
  });

  it('throws when active provider has no configured slice', () => {
    const s = baseSettings();
    s.activeProvider = 'openai-compat';
    s.providers['openai-compat'] = null;
    expect(() => toV2Wire(s)).toThrow(/no configured slice/);
  });
});

describe('postSettingsV2', () => {
  beforeEach(() => {
    setBackendPortForTesting(9999);
  });

  it('POSTs the wire to /settings/v2', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify({ status: 'ok' }), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    await postSettingsV2(baseSettings());
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0]!;
    expect(url).toMatch(/\/settings\/v2$/);
    expect(init.method).toBe('POST');
    expect((init.headers as Record<string, string>)['content-type']).toBe('application/json');
    const body = JSON.parse(init.body as string);
    expect(body.chat.active_provider_id).toBe('anthropic');
  });

  it('throws ChatError on non-2xx with structured envelope', async () => {
    vi.stubGlobal(
      'fetch',
      vi
        .fn()
        .mockResolvedValue(
          new Response(JSON.stringify({ error: { message: 'invalid api_key' } }), { status: 400 }),
        ),
    );
    await expect(postSettingsV2(baseSettings())).rejects.toThrow(/invalid api_key/);
  });
});

// Silence unused import warning - useStore is available for future local-mistralrs tests
void useStore;
