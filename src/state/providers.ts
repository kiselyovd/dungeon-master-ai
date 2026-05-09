/**
 * Provider configuration model. The discriminated union here is the single
 * source of truth for "what does the user have configured" - the settings
 * slice (B5/C1) stores it, the SettingsForm (C1) edits it, the streamChat
 * envelope (C4) ships it to the Rust sidecar.
 *
 * `local-mistralrs` is reserved as a slot for the M4 milestone (embedded
 * inference engine). M1.5 ships only `anthropic` and `openai-compat` UIs;
 * the type stays in the union so a future addition is just an extra branch
 * in every `switch (kind)` (TS exhaustiveness catches misses).
 */

import * as v from 'valibot';

// ---- Branded primitives -------------------------------------------------

declare const apiKeyBrand: unique symbol;
export type ApiKey = string & { readonly [apiKeyBrand]: never };

declare const baseUrlBrand: unique symbol;
export type BaseUrl = string & { readonly [baseUrlBrand]: never };

const TRIMMED_NONEMPTY = /^\S(?:.*\S)?$/;

/**
 * Accept any non-empty trimmed string as an API key. We deliberately do NOT
 * validate provider-specific prefixes (e.g. `sk-ant-`) here because users may
 * paste keys for openai-compat targets where prefix conventions differ.
 */
export function parseApiKey(input: string): ApiKey | null {
  const trimmed = input.trim();
  if (!TRIMMED_NONEMPTY.test(trimmed)) return null;
  return trimmed as ApiKey;
}

/**
 * Validate a base URL using the WHATWG URL parser. Accepts http(s) only;
 * rejects anything else so we don't accidentally fire requests at file://
 * or chrome-extension://.
 */
export function parseBaseUrl(input: string): BaseUrl | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  try {
    const url = new URL(trimmed);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    return trimmed as BaseUrl;
  } catch {
    return null;
  }
}

// ---- Provider config union ---------------------------------------------

export type ProviderKind = 'anthropic' | 'openai-compat' | 'local-mistralrs';

export interface AnthropicConfig {
  kind: 'anthropic';
  apiKey: ApiKey;
  model: string;
}

export interface OpenaiCompatConfig {
  kind: 'openai-compat';
  baseUrl: BaseUrl;
  apiKey: ApiKey;
  model: string;
}

/** Slot for M4. Settings UI will not surface this until then. */
export interface LocalMistralRsConfig {
  kind: 'local-mistralrs';
  modelPath: string;
  contextWindow: number;
}

export type ProviderConfig = AnthropicConfig | OpenaiCompatConfig | LocalMistralRsConfig;

// ---- Schemas (valibot) --------------------------------------------------

const ApiKeySchema = v.pipe(
  v.string(),
  v.transform((s) => s.trim()),
  v.minLength(1),
);

const BaseUrlSchema = v.pipe(
  v.string(),
  v.transform((s) => s.trim()),
  v.url(),
  v.check((s) => s.startsWith('http://') || s.startsWith('https://'), 'http(s) only'),
);

export const AnthropicConfigSchema = v.object({
  kind: v.literal('anthropic'),
  apiKey: ApiKeySchema,
  model: v.pipe(v.string(), v.minLength(1)),
});

export const OpenaiCompatConfigSchema = v.object({
  kind: v.literal('openai-compat'),
  baseUrl: BaseUrlSchema,
  apiKey: ApiKeySchema,
  model: v.pipe(v.string(), v.minLength(1)),
});

export const LocalMistralRsConfigSchema = v.object({
  kind: v.literal('local-mistralrs'),
  modelPath: v.pipe(v.string(), v.minLength(1)),
  contextWindow: v.pipe(v.number(), v.minValue(512)),
});

export const ProviderConfigSchema = v.variant('kind', [
  AnthropicConfigSchema,
  OpenaiCompatConfigSchema,
  LocalMistralRsConfigSchema,
]);

// ---- Defaults / helpers ------------------------------------------------

export const DEFAULT_ANTHROPIC_MODEL = 'claude-haiku-4-5-20251001';

/**
 * Default context window for the embedded local mistralrs provider. Qwen3.5
 * GGUFs ship with 32k native context, but mistralrs-server defaults to 8k
 * on cold-start unless overridden; we mirror that to avoid silent truncation.
 */
export const DEFAULT_LOCAL_CONTEXT_WINDOW = 8192;

/** Convenient `assertNever` for exhaustive switches over ProviderKind. */
export function assertNeverProvider(_value: never): never {
  throw new Error(`unhandled provider kind: ${String(_value)}`);
}
