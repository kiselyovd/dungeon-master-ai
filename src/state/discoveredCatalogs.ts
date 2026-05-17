/**
 * Cache for `POST /providers/discover` results. Models are public information
 * (not secrets) so this slice persists to the plaintext settings.json LazyStore,
 * NOT to the Stronghold secrets vault.
 *
 * Cache invalidation:
 * - 7-day TTL on `cachedAt`.
 * - `cacheKey` rotation: derived from `sha256(provider_id + ":" + base_url +
 *   ":" + api_key)`. Any field change (key rotation, base URL switch) yields a
 *   new key and the prior entry becomes unreachable on next discover.
 */
import type { ProviderKind } from './providers';

export const CACHE_TTL_MS = 7 * 24 * 60 * 60 * 1000;

export type ModelSource = 'curated' | 'discovered-api' | 'discovered-hf-hub' | 'custom-hf';

export interface Capabilities {
  vision_input: boolean;
  reasoning: boolean;
  tool_calls: boolean;
  streaming: boolean;
}

export interface ResolvedModelEntry {
  model_id: string;
  display_name: string;
  capabilities: Capabilities;
  source: ModelSource;
  context_length?: number | null;
  price_per_million_input?: number | null;
  price_per_million_output?: number | null;
}

export interface DiscoveredCatalog {
  cacheKey: string;
  cachedAt: string;
  source: ModelSource;
  models: ResolvedModelEntry[];
  next_cursor?: string | null;
}

export type DiscoveredCatalogsMap = Partial<Record<ProviderKind, DiscoveredCatalog | null>>;

export interface CacheKeyInput {
  providerId: string;
  baseUrl?: string;
  apiKey?: string;
}

export async function deriveCacheKey(input: CacheKeyInput): Promise<string> {
  const text = `${input.providerId}:${input.baseUrl ?? ''}:${input.apiKey ?? ''}`;
  const bytes = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

export function isCacheFresh(catalog: DiscoveredCatalog, nowMs: number): boolean {
  const cachedMs = Date.parse(catalog.cachedAt);
  if (Number.isNaN(cachedMs)) return false;
  return nowMs - cachedMs < CACHE_TTL_MS;
}
