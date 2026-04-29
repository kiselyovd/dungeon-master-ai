import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { ApiKey, BaseUrl } from '../../state/providers';
import { ChatError } from '../errors';
import { getProviders, postSettings } from '../providers';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

beforeEach(() => {
  vi.unstubAllGlobals();
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe('getProviders', () => {
  it('returns the parsed JSON shape', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(
            JSON.stringify({
              available: ['anthropic', 'openai-compat'],
              active: { kind: 'anthropic', default_model: 'claude-haiku' },
            }),
            { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
          ),
      ),
    );
    const info = await getProviders();
    expect(info.available).toContain('anthropic');
    expect(info.active.kind).toBe('anthropic');
  });

  it('throws ChatError on non-2xx', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 503 })),
    );
    await expect(getProviders()).rejects.toBeInstanceOf(ChatError);
  });
});

describe('postSettings', () => {
  it('translates Anthropic config to snake_case wire format', async () => {
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit) => {
        calls.push({ url, init });
        return new Response(JSON.stringify({ kind: 'anthropic', default_model: 'claude-haiku' }), {
          status: 200,
          headers: new Headers({ 'content-type': 'application/json' }),
        });
      }),
    );

    await postSettings({
      kind: 'anthropic',
      apiKey: 'sk-ant-abc' as ApiKey,
      model: 'claude-haiku',
    });

    expect(calls).toHaveLength(1);
    const sent = JSON.parse(calls[0]?.init.body as string) as Record<string, unknown>;
    expect(sent).toEqual({ kind: 'anthropic', api_key: 'sk-ant-abc', model: 'claude-haiku' });
  });

  it('translates OpenAI-compat config including base_url', async () => {
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit) => {
        calls.push({ url, init });
        return new Response(
          JSON.stringify({ kind: 'openai-compat', default_model: 'qwen3-1.7b' }),
          { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
        );
      }),
    );

    await postSettings({
      kind: 'openai-compat',
      baseUrl: 'http://localhost:1234/v1' as BaseUrl,
      apiKey: 'sk-test' as ApiKey,
      model: 'qwen3-1.7b',
    });

    const sent = JSON.parse(calls[0]?.init.body as string) as Record<string, unknown>;
    expect(sent).toEqual({
      kind: 'openai-compat',
      base_url: 'http://localhost:1234/v1',
      api_key: 'sk-test',
      model: 'qwen3-1.7b',
    });
  });

  it('throws ChatError on backend rejection', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(JSON.stringify({ error: { code: 'bad_request', message: 'nope' } }), {
            status: 400,
            headers: new Headers({ 'content-type': 'application/json' }),
          }),
      ),
    );

    await expect(
      postSettings({ kind: 'anthropic', apiKey: 'sk-ant' as ApiKey, model: 'claude' }),
    ).rejects.toBeInstanceOf(ChatError);
  });
});
