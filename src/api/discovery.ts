/**
 * Frontend client for `POST /providers/discover`. Returns a normalised
 * DiscoveryResponse parsed via valibot so a backend shape drift surfaces as a
 * single `invalid_response` ChatError instead of a deep runtime crash.
 *
 * Error mapping:
 * - fetch() throws (network, DNS, CORS, AbortError) -> ChatError via ChatError.from
 * - HTTP 401 -> ChatError('auth_failed')
 * - HTTP 429 -> ChatError('rate_limit')
 * - HTTP 4xx/5xx (other) -> ChatError('provider_error')
 * - 2xx body fails schema -> ChatError('invalid_response')
 */
import * as v from 'valibot';
import type {
  DiscoveredCatalog,
  ModelSource,
  ResolvedModelEntry,
} from '../state/discoveredCatalogs';
import { backendUrl } from './client';
import { ChatError } from './errors';

const ModelSourceSchema = v.picklist([
  'curated',
  'discovered-api',
  'discovered-hf-hub',
  'custom-hf',
] as const);

const CapabilitiesSchema = v.object({
  vision_input: v.boolean(),
  reasoning: v.boolean(),
  tool_calls: v.boolean(),
  streaming: v.boolean(),
});

const ResolvedModelEntrySchema = v.object({
  model_id: v.pipe(v.string(), v.minLength(1)),
  display_name: v.pipe(v.string(), v.minLength(1)),
  capabilities: CapabilitiesSchema,
  source: ModelSourceSchema,
  context_length: v.optional(v.nullable(v.number())),
  price_per_million_input: v.optional(v.nullable(v.number())),
  price_per_million_output: v.optional(v.nullable(v.number())),
});

export const DiscoveryResponseSchema = v.object({
  models: v.array(ResolvedModelEntrySchema),
  cached_at: v.string(),
  source: ModelSourceSchema,
  next_cursor: v.optional(v.nullable(v.string())),
});

export interface DiscoverParams {
  provider_id: string;
  base_url?: string | undefined;
  api_key?: string | undefined;
  search_query?: string | undefined;
  cursor?: string | undefined;
}

export interface DiscoveryResponse {
  models: ResolvedModelEntry[];
  cached_at: string;
  source: ModelSource;
  next_cursor?: string | null;
}

function bodyForRequest(params: DiscoverParams): string {
  const out: Record<string, unknown> = { provider_id: params.provider_id };
  if (params.base_url !== undefined) out.base_url = params.base_url;
  if (params.api_key !== undefined) out.api_key = params.api_key;
  if (params.search_query !== undefined) out.search_query = params.search_query;
  if (params.cursor !== undefined) out.cursor = params.cursor;
  return JSON.stringify(out);
}

export async function postDiscover(params: DiscoverParams): Promise<DiscoveryResponse> {
  const url = await backendUrl('/providers/discover');
  let resp: Response;
  try {
    resp = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: bodyForRequest(params),
    });
  } catch (e) {
    throw ChatError.from(e);
  }
  if (!resp.ok) {
    if (resp.status === 401) {
      throw new ChatError('auth_failed', `discover: unauthorized (401)`);
    }
    if (resp.status === 429) {
      throw new ChatError('rate_limit', `discover: rate limited (429)`);
    }
    throw new ChatError('provider_error', `discover: HTTP ${resp.status}`);
  }
  const json = (await resp.json().catch(() => null)) as unknown;
  const parsed = v.safeParse(DiscoveryResponseSchema, json);
  if (!parsed.success) {
    throw new ChatError('invalid_response', 'discover: response shape invalid');
  }
  return parsed.output as DiscoveryResponse;
}

export type { DiscoveredCatalog, ModelSource, ResolvedModelEntry };
