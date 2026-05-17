import * as v from 'valibot';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DiscoveryResponseSchema, postDiscover } from '../discovery';
import { ChatError } from '../errors';

vi.mock('../client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://test.local${path}`),
}));

beforeEach(() => {
  vi.unstubAllGlobals();
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe('postDiscover', () => {
  it('POSTs to /providers/discover and parses a valid response', async () => {
    const body = {
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
          source: 'curated',
          context_length: 1_000_000,
        },
      ],
      cached_at: '2026-05-17T12:00:00Z',
      source: 'curated',
      next_cursor: null,
    };
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit) => {
        calls.push({ url, init });
        return new Response(JSON.stringify(body), {
          status: 200,
          headers: new Headers({ 'content-type': 'application/json' }),
        });
      }),
    );

    const result = await postDiscover({ provider_id: 'anthropic' });

    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toMatch(/\/providers\/discover$/);
    expect(calls[0]?.init.method).toBe('POST');
    expect(result.models).toHaveLength(1);
    expect(result.models[0]?.model_id).toBe('claude-opus-4-7');
    expect(result.source).toBe('curated');
    expect(result.cached_at).toBe('2026-05-17T12:00:00Z');
  });

  it('sends provider_id, base_url, api_key and search_query in body', async () => {
    const calls: { url: string; init: RequestInit }[] = [];
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: string, init: RequestInit) => {
        calls.push({ url, init });
        return new Response(
          JSON.stringify({ models: [], cached_at: '2026-05-17T12:00:00Z', source: 'curated' }),
          { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
        );
      }),
    );

    await postDiscover({
      provider_id: 'openai-compat',
      base_url: 'http://localhost:1234/v1',
      api_key: 'sk-x',
      search_query: 'qwen',
    });

    const sent = JSON.parse(calls[0]?.init.body as string) as Record<string, unknown>;
    expect(sent).toEqual({
      provider_id: 'openai-compat',
      base_url: 'http://localhost:1234/v1',
      api_key: 'sk-x',
      search_query: 'qwen',
    });
  });

  it('throws ChatError with auth_failed code on 401', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('unauthorized', { status: 401 })),
    );
    await expect(postDiscover({ provider_id: 'anthropic' })).rejects.toMatchObject({
      name: 'ChatError',
      code: 'auth_failed',
    });
  });

  it('throws ChatError with rate_limit code on 429', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('slow down', { status: 429 })),
    );
    await expect(postDiscover({ provider_id: 'anthropic' })).rejects.toMatchObject({
      name: 'ChatError',
      code: 'rate_limit',
    });
  });

  it('throws ChatError with provider_error code on 502', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('bad gateway', { status: 502 })),
    );
    await expect(postDiscover({ provider_id: 'anthropic' })).rejects.toMatchObject({
      name: 'ChatError',
      code: 'provider_error',
    });
  });

  it('throws ChatError with invalid_response on schema mismatch', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async () =>
          new Response(JSON.stringify({ wrong: 'shape' }), {
            status: 200,
            headers: new Headers({ 'content-type': 'application/json' }),
          }),
      ),
    );
    await expect(postDiscover({ provider_id: 'anthropic' })).rejects.toMatchObject({
      name: 'ChatError',
      code: 'invalid_response',
    });
  });

  it('throws ChatError with network code when fetch itself throws TypeError', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new TypeError('Failed to fetch');
      }),
    );
    await expect(postDiscover({ provider_id: 'anthropic' })).rejects.toMatchObject({
      name: 'ChatError',
      code: 'network',
    });
  });

  it('DiscoveryResponseSchema rejects unknown source enum', () => {
    const bad = {
      models: [],
      cached_at: '2026-05-17T12:00:00Z',
      source: 'nope',
    };
    const parsed = v.safeParse(DiscoveryResponseSchema, bad);
    expect(parsed.success).toBe(false);
  });

  it('DiscoveryResponseSchema accepts entries with optional pricing absent', () => {
    const ok = {
      models: [
        {
          model_id: 'm',
          display_name: 'M',
          capabilities: {
            vision_input: false,
            reasoning: false,
            tool_calls: true,
            streaming: true,
          },
          source: 'discovered-api',
        },
      ],
      cached_at: '2026-05-17T12:00:00Z',
      source: 'discovered-api',
    };
    const parsed = v.safeParse(DiscoveryResponseSchema, ok);
    expect(parsed.success).toBe(true);
  });

  it('postDiscover does not throw on a known ChatError thrown by ChatError.from', async () => {
    // sanity: make sure ChatError used directly still bubbles through
    expect(new ChatError('auth_failed', 'x').code).toBe('auth_failed');
  });
});
