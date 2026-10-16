/**
 * Hook around POST /providers/discover.
 *
 * State machine: idle -> loading -> idle (success) or idle -> loading -> error.
 *
 * The hook does NOT auto-discover on mount (spec anti-decision: "no background
 * discovery, only on explicit button"). On mount it derives the current
 * cacheKey from the inputs and exposes the slice-cached catalog ONLY if both:
 *   1. cached.cacheKey === current key, AND
 *   2. isCacheFresh(cached, now) is true (7-day TTL).
 *
 * On `discover()` success the catalog is written into the settings slice via
 * setDiscoveredCatalog so it persists to settings.json (Task 4).
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { ModelSource } from '../api/discovery';
import { postDiscover } from '../api/discovery';
import {
  type DiscoveredCatalog,
  deriveCacheKey,
  isCacheFresh,
  type ResolvedModelEntry,
} from '../state/discoveredCatalogs';
import type { ProviderKind } from '../state/providers';
import { useStore } from '../state/useStore';

export type DiscoverStatus = 'idle' | 'loading' | 'error';

export interface UseDiscoverProviderInput {
  providerId: ProviderKind;
  baseUrl?: string;
  apiKey?: string;
}

export interface UseDiscoverProviderResult {
  status: DiscoverStatus;
  models: ResolvedModelEntry[];
  error: string | null;
  lastCachedAt: string | null;
  discover: () => Promise<ModelSource | null>;
}

export function useDiscoverProvider(input: UseDiscoverProviderInput): UseDiscoverProviderResult {
  const { providerId, baseUrl, apiKey } = input;
  const inputSignature = JSON.stringify(input);
  const currentInput = useRef(inputSignature);
  currentInput.current = inputSignature;
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const cached = useStore((s) => s.settings.discoveredCatalogs[providerId] ?? null);
  const setDiscoveredCatalog = useStore((s) => s.settings.setDiscoveredCatalog);

  const [status, setStatus] = useState<DiscoverStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [cacheKey, setCacheKey] = useState<string | null>(null);
  useEffect(() => {
    if (currentInput.current !== inputSignature) return;
    setStatus('idle');
    setError(null);
  }, [inputSignature]);

  useEffect(() => {
    let alive = true;
    deriveCacheKey({ providerId, baseUrl, apiKey }).then((k) => {
      if (alive) setCacheKey(k);
    });
    return () => {
      alive = false;
    };
  }, [providerId, baseUrl, apiKey]);

  const usableCache = useMemo<DiscoveredCatalog | null>(() => {
    if (!cached || !cacheKey) return null;
    if (cached.cacheKey !== cacheKey) return null;
    if (!isCacheFresh(cached, Date.now())) return null;
    return cached;
  }, [cached, cacheKey]);

  const discover = useCallback(async () => {
    setStatus('loading');
    setError(null);
    try {
      const resp = await postDiscover({
        provider_id: providerId,
        base_url: baseUrl,
        api_key: apiKey,
      });
      const key = await deriveCacheKey({ providerId, baseUrl, apiKey });
      if (!mounted.current || currentInput.current !== inputSignature) return null;
      const catalog: DiscoveredCatalog = {
        cacheKey: key,
        cachedAt: resp.cached_at,
        source: resp.source,
        models: resp.models,
        next_cursor: resp.next_cursor ?? null,
      };
      setDiscoveredCatalog(providerId, catalog);
      setStatus('idle');
      return resp.source;
    } catch (err) {
      if (!mounted.current || currentInput.current !== inputSignature) return null;
      const msg = err instanceof Error ? err.message : 'discover failed';
      setError(msg);
      setStatus('error');
      return null;
    }
  }, [providerId, baseUrl, apiKey, setDiscoveredCatalog, inputSignature]);

  return {
    status,
    models: usableCache?.models ?? [],
    error,
    lastCachedAt: usableCache?.cachedAt ?? null,
    discover,
  };
}
