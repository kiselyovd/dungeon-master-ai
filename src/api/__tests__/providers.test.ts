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
              available: ['openai-compat', 'local-mistralrs'],
              active: { kind: 'openai-compat', default_model: 'anthropic/claude-haiku' },
            }),
            { status: 200, headers: new Headers({ 'content-type': 'application/json' }) },
          ),
      ),
    );
    const info = await getProviders();
    expect(info.available).toContain('openai-compat');
    expect(info.active.kind).toBe('openai-compat');
  });

  it('throws ChatError on non-2xx', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('', { status: 503 })),
    );
    await expect(getProviders()).rejects.toBeInstanceOf(ChatError);
  });
});
