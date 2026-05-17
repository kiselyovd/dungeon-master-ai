import { describe, expect, it } from 'vitest';
import {
  CACHE_TTL_MS,
  type DiscoveredCatalog,
  deriveCacheKey,
  isCacheFresh,
} from '../discoveredCatalogs';

describe('deriveCacheKey', () => {
  it('hashes provider_id alone when base_url and api_key are empty', async () => {
    const key = await deriveCacheKey({ providerId: 'anthropic' });
    expect(key).toMatch(/^[0-9a-f]{64}$/);
  });

  it('returns identical key for identical inputs', async () => {
    const a = await deriveCacheKey({
      providerId: 'openai-compat',
      baseUrl: 'http://localhost:1234/v1',
      apiKey: 'sk-test',
    });
    const b = await deriveCacheKey({
      providerId: 'openai-compat',
      baseUrl: 'http://localhost:1234/v1',
      apiKey: 'sk-test',
    });
    expect(a).toBe(b);
  });

  it('returns different keys when api_key changes', async () => {
    const a = await deriveCacheKey({
      providerId: 'anthropic',
      apiKey: 'sk-1',
    });
    const b = await deriveCacheKey({
      providerId: 'anthropic',
      apiKey: 'sk-2',
    });
    expect(a).not.toBe(b);
  });

  it('returns different keys when base_url changes', async () => {
    const a = await deriveCacheKey({
      providerId: 'openai-compat',
      baseUrl: 'http://localhost:1234/v1',
      apiKey: 'k',
    });
    const b = await deriveCacheKey({
      providerId: 'openai-compat',
      baseUrl: 'http://localhost:5678/v1',
      apiKey: 'k',
    });
    expect(a).not.toBe(b);
  });
});

describe('isCacheFresh', () => {
  it('returns true for a catalog cached now', () => {
    const cat: DiscoveredCatalog = {
      cacheKey: 'k',
      cachedAt: new Date().toISOString(),
      source: 'curated',
      models: [],
    };
    expect(isCacheFresh(cat, Date.now())).toBe(true);
  });

  it('returns false for a catalog older than 7 days', () => {
    const eightDaysAgo = new Date(Date.now() - 8 * 24 * 60 * 60 * 1000);
    const cat: DiscoveredCatalog = {
      cacheKey: 'k',
      cachedAt: eightDaysAgo.toISOString(),
      source: 'curated',
      models: [],
    };
    expect(isCacheFresh(cat, Date.now())).toBe(false);
  });

  it('returns true for a catalog cached just under 7 days ago', () => {
    const justUnder = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000 + 1000);
    const cat: DiscoveredCatalog = {
      cacheKey: 'k',
      cachedAt: justUnder.toISOString(),
      source: 'curated',
      models: [],
    };
    expect(isCacheFresh(cat, Date.now())).toBe(true);
  });

  it('returns false for a catalog with unparseable cachedAt', () => {
    const cat: DiscoveredCatalog = {
      cacheKey: 'k',
      cachedAt: 'not-a-date',
      source: 'curated',
      models: [],
    };
    expect(isCacheFresh(cat, Date.now())).toBe(false);
  });
});

describe('CACHE_TTL_MS', () => {
  it('is 7 days in milliseconds', () => {
    expect(CACHE_TTL_MS).toBe(7 * 24 * 60 * 60 * 1000);
  });
});
