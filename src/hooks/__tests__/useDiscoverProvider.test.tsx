import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type DiscoveryResponse, postDiscover } from '../../api/discovery';
import { useStore } from '../../state/useStore';
import { useDiscoverProvider } from '../useDiscoverProvider';

vi.mock('../../api/discovery', () => ({
  postDiscover: vi.fn(),
}));

const postDiscoverMock = vi.mocked(postDiscover);

async function flushMount() {
  await act(async () => {
    await Promise.resolve();
  });
}

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  postDiscoverMock.mockReset();
});

afterEach(() => {
  useStore.setState(useStore.getInitialState());
});

describe('useDiscoverProvider', () => {
  it('starts idle with no models when no cache present', async () => {
    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'k',
      }),
    );
    await flushMount();
    expect(result.current.status).toBe('idle');
    expect(result.current.models).toEqual([]);
    expect(result.current.error).toBeNull();
    expect(result.current.lastCachedAt).toBeNull();
  });

  it('does NOT auto-discover on mount', async () => {
    renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'k',
      }),
    );
    await flushMount();
    expect(postDiscoverMock).not.toHaveBeenCalled();
  });

  it('discover() success transitions idle -> loading -> idle with models', async () => {
    const resp: DiscoveryResponse = {
      models: [
        {
          model_id: 'gpt-4o',
          display_name: 'GPT-4o',
          capabilities: {
            vision_input: true,
            reasoning: false,
            tool_calls: true,
            streaming: true,
          },
          source: 'discovered-api',
        },
      ],
      cached_at: new Date().toISOString(),
      source: 'discovered-api',
      next_cursor: null,
    };
    postDiscoverMock.mockResolvedValueOnce(resp);

    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'sk-x',
      }),
    );
    await flushMount();

    await act(async () => {
      await result.current.discover();
    });

    expect(result.current.status).toBe('idle');
    expect(result.current.models).toHaveLength(1);
    expect(result.current.models[0]?.model_id).toBe('gpt-4o');
    expect(result.current.error).toBeNull();
  });

  it('discover() error transitions idle -> loading -> error', async () => {
    postDiscoverMock.mockRejectedValueOnce(new Error('boom'));

    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'bad',
      }),
    );
    await flushMount();

    await act(async () => {
      await result.current.discover();
    });

    expect(result.current.status).toBe('error');
    expect(result.current.error).toMatch(/boom/i);
    expect(result.current.models).toEqual([]);
  });

  it('discover() success writes the catalog into the settings slice', async () => {
    const resp: DiscoveryResponse = {
      models: [],
      cached_at: '2026-05-17T12:00:00Z',
      source: 'curated',
      next_cursor: null,
    };
    postDiscoverMock.mockResolvedValueOnce(resp);

    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'k',
      }),
    );
    await flushMount();

    await act(async () => {
      await result.current.discover();
    });

    const stored = useStore.getState().settings.discoveredCatalogs['openai-compat'];
    expect(stored).not.toBeNull();
    expect(stored?.cachedAt).toBe('2026-05-17T12:00:00Z');
    expect(stored?.cacheKey).toMatch(/^[0-9a-f]{64}$/);
  });

  it('ignores stale (>7d) cached entries', async () => {
    const eightDaysAgo = new Date(Date.now() - 8 * 24 * 60 * 60 * 1000).toISOString();
    useStore.setState((s) => ({
      settings: {
        ...s.settings,
        discoveredCatalogs: {
          'openai-compat': {
            cacheKey: 'whatever',
            cachedAt: eightDaysAgo,
            source: 'curated',
            models: [
              {
                model_id: 'stale',
                display_name: 'Stale',
                capabilities: {
                  vision_input: false,
                  reasoning: false,
                  tool_calls: true,
                  streaming: true,
                },
                source: 'curated',
              },
            ],
          },
        },
      },
    }));

    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'k',
      }),
    );
    await waitFor(() => {
      expect(result.current.models).toEqual([]);
    });
    expect(result.current.status).toBe('idle');
  });

  it('ignores cached entries whose cacheKey does not match the current inputs', async () => {
    const resp: DiscoveryResponse = {
      models: [
        {
          model_id: 'one',
          display_name: 'One',
          capabilities: {
            vision_input: false,
            reasoning: false,
            tool_calls: true,
            streaming: true,
          },
          source: 'curated',
        },
      ],
      cached_at: new Date().toISOString(),
      source: 'curated',
      next_cursor: null,
    };
    postDiscoverMock.mockResolvedValueOnce(resp);

    // Seed cache by running discover with apiKey=A
    const { result: seed } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'A',
      }),
    );
    await flushMount();
    await act(async () => {
      await seed.current.discover();
    });
    expect(seed.current.models).toHaveLength(1);

    // New hook with apiKey=B - cached entry has different cacheKey, must NOT surface
    const { result: rotated } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'B',
      }),
    );
    await waitFor(() => {
      expect(rotated.current.models).toEqual([]);
    });
  });

  it('exposes lastCachedAt from a fresh cached entry that matches the cache key', async () => {
    // Relative timestamp: the hook only surfaces a cache entry within the
    // 7-day freshness TTL, so a hardcoded date here is a time-bomb that
    // fails once the calendar advances past it.
    const freshCachedAt = new Date().toISOString();
    const resp: DiscoveryResponse = {
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
          source: 'curated',
        },
      ],
      cached_at: freshCachedAt,
      source: 'curated',
      next_cursor: null,
    };
    postDiscoverMock.mockResolvedValueOnce(resp);

    const { result } = renderHook(() =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1234/v1',
        apiKey: 'k',
      }),
    );
    await flushMount();
    await act(async () => {
      await result.current.discover();
    });

    expect(result.current.lastCachedAt).toBe(freshCachedAt);
  });
});
