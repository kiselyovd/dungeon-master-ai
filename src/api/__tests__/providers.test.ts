import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ChatError } from '../errors';
import { getProviders } from '../providers';

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
