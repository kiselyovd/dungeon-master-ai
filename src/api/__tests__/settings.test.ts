import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SettingsData } from '../../state/settings';
import { setBackendPortForTesting } from '../client';
import { ensureSettingsV2, invalidateBackendSettings, postSettingsV2, toV2Wire } from '../settings';

// Read whatever DEFAULT_SETTINGS exists; if not exported, build a baseline inline.
function baseSettings(): SettingsData {
  return {
    activeProvider: 'openai-compat',
    providers: {
      'openai-compat': {
        kind: 'openai-compat',
        baseUrl: 'https://openrouter.ai/api/v1' as never,
        apiKey: 'sk-test' as never,
        model: 'anthropic/claude-3.5-sonnet',
      },
      'local-mistralrs': null,
    },
    uiLanguage: 'en',
    narrationLanguage: 'en',
    systemPrompt: 'DM',
    temperature: 0.7,
    replicateApiKey: null,
    chatPanelWidth: 480,
    readingFontSize: 17,
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
    providerMigrationNotice: false,
  } as SettingsData;
}

describe('toV2Wire', () => {
  it('maps the openai-compat baseline correctly', () => {
    const wire = toV2Wire(baseSettings());
    expect(wire.chat.active_provider_id).toBe('openai-compat');
    expect(wire.chat.active_model_id).toBe('anthropic/claude-3.5-sonnet');
    expect(wire.chat.providers).toEqual({
      'openai-compat': {
        base_url: 'https://openrouter.ai/api/v1',
        api_key: 'sk-test',
      },
    });
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

  it('emits replicate api_key in image.providers when preset is cloud', () => {
    const s = baseSettings();
    s.imagePreset = 'cloud';
    s.replicateApiKey = 'r8-test-key';
    const wire = toV2Wire(s);
    expect(wire.image.active_provider_id).toBe('replicate');
    expect(wire.image.providers).toEqual({ replicate: { api_key: 'r8-test-key' } });
  });

  it('emits empty image.providers when cloud preset selected but replicate key is null', () => {
    const s = baseSettings();
    s.imagePreset = 'cloud';
    s.replicateApiKey = null;
    const wire = toV2Wire(s);
    expect(wire.image.providers).toEqual({});
  });

  it('emits empty image.providers for non-cloud presets even with replicate key set', () => {
    const s = baseSettings();
    s.imagePreset = 'balanced';
    s.replicateApiKey = 'r8-test-key';
    const wire = toV2Wire(s);
    expect(wire.image.providers).toEqual({});
  });
});

describe('postSettingsV2', () => {
  beforeEach(() => {
    setBackendPortForTesting(9999);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
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
    expect(body.chat.active_provider_id).toBe('openai-compat');
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

describe('acknowledged backend configuration', () => {
  beforeEach(() => {
    invalidateBackendSettings();
    setBackendPortForTesting(9999);
  });
  afterEach(() => vi.unstubAllGlobals());
  it('deduplicates successful restores but writes again for a new backend', async () => {
    const fetch = vi.fn().mockImplementation(async () => new Response('{}'));
    vi.stubGlobal('fetch', fetch);
    await ensureSettingsV2(baseSettings());
    await ensureSettingsV2(baseSettings());
    expect(fetch).toHaveBeenCalledTimes(1);
    setBackendPortForTesting(9998);
    await ensureSettingsV2(baseSettings());
    expect(fetch).toHaveBeenCalledTimes(2);
  });
  it('does not cache a failed restore and keeps manual Save independent', async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(new Response('{}', { status: 500 }))
      .mockImplementation(async () => new Response('{}'));
    vi.stubGlobal('fetch', fetch);
    await expect(ensureSettingsV2(baseSettings())).rejects.toThrow();
    await ensureSettingsV2(baseSettings());
    await ensureSettingsV2(baseSettings());
    expect(fetch).toHaveBeenCalledTimes(2);
    await postSettingsV2(baseSettings());
    expect(fetch).toHaveBeenCalledTimes(3);
  });
  it('serializes immutable snapshots so a late old write cannot replace a newer Save', async () => {
    let resolve!: (r: Response) => void;
    const fetch = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<Response>((r) => {
            resolve = r;
          }),
      )
      .mockImplementation(async () => new Response('{}'));
    vi.stubGlobal('fetch', fetch);
    const settings = baseSettings();
    const old = postSettingsV2(settings);
    await new Promise((r) => setTimeout(r, 0));
    const config = settings.providers['openai-compat'];
    if (!config) throw new Error('missing fixture');
    config.model = 'new-model';
    const latest = postSettingsV2(settings);
    expect(fetch).toHaveBeenCalledTimes(1);
    resolve(new Response('{}'));
    await Promise.all([old, latest]);
    expect(JSON.parse(fetch.mock.calls[0]?.[1].body).chat.active_model_id).toBe(
      'anthropic/claude-3.5-sonnet',
    );
    expect(JSON.parse(fetch.mock.calls[1]?.[1].body).chat.active_model_id).toBe('new-model');
  });
  it('an acknowledgement from before backend invalidation cannot satisfy the next restore', async () => {
    let resolve!: (r: Response) => void;
    const fetch = vi
      .fn()
      .mockImplementationOnce(
        () =>
          new Promise<Response>((r) => {
            resolve = r;
          }),
      )
      .mockImplementation(async () => new Response('{}'));
    vi.stubGlobal('fetch', fetch);
    const old = ensureSettingsV2(baseSettings());
    await new Promise((r) => setTimeout(r, 0));
    invalidateBackendSettings();
    const current = ensureSettingsV2(baseSettings());
    resolve(new Response('{}'));
    const results = await Promise.allSettled([old, current]);
    expect(results[0]?.status).toBe('rejected');
    expect(results[1]?.status).toBe('fulfilled');
    expect(fetch).toHaveBeenCalledTimes(2);
  });
});

it('a hung settings request times out and releases the FIFO for another restore', async () => {
  invalidateBackendSettings();
  setBackendPortForTesting(9999);
  vi.useFakeTimers();
  try {
    const fetch = vi
      .fn()
      .mockImplementationOnce(
        (_url, init: RequestInit) =>
          new Promise<Response>((_resolve, reject) => {
            init.signal?.addEventListener(
              'abort',
              () => reject(new DOMException('aborted', 'AbortError')),
              { once: true },
            );
          }),
      )
      .mockImplementation(async () => new Response('{}'));
    vi.stubGlobal('fetch', fetch);
    const first = ensureSettingsV2(baseSettings());
    const rejection = expect(first).rejects.toMatchObject({
      code: 'network',
      message: 'provider_configuration_timeout',
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(fetch).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(10_000);
    await rejection;
    await ensureSettingsV2(baseSettings());
    expect(fetch).toHaveBeenCalledTimes(2);
  } finally {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  }
});
