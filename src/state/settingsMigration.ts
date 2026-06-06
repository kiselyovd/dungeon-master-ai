/**
 * ## Role
 *
 * `SettingsConfigV2` is the WIRE SHAPE for two things:
 * 1. The body of `POST /settings/v2` (sent by SettingsModal on save).
 * 2. The input/output of the legacy v1 -> v2 migration on first boot.
 *
 * For the runtime app state used by components, see `SettingsData` in
 * `./settings.ts` (flat, the canonical type for the UI).
 */

/**
 * M7-DM hard-cutover settings migration.
 *
 * Reads the persisted settings JSON, returns a normalised v2 shape with
 * defaults filled in for new fields. Handles three input shapes:
 *
 *  - **Fresh install** (`null` / `undefined`) → returns DEFAULTS_V2,
 *    `didReset: false`.
 *  - **Legacy v1** (has top-level `activeProvider` + `providers` keys) → maps
 *    to v2, returns the merged shape with `didReset: false`.
 *  - **Corrupt / unknown** (not an object, or throws during normalisation) →
 *    returns DEFAULTS_V2 with `didReset: true` so App.tsx can fire a toast.
 *
 * "Hard cutover" semantics: after this runs once, the persisted JSON is
 * rewritten in v2 shape and old top-level fields disappear.
 */

export type ReasoningBudget = 'low' | 'medium' | 'high';
export type ImagePreset = 'fast' | 'balanced' | 'quality' | 'quality-oss' | 'cloud';
export type VideoMode = 'prerecorded' | 'live' | 'race';
export type SceneTransitions = 'auto' | 'manual' | 'off';

export type Capabilities = {
  vision_input: boolean;
  reasoning: boolean;
  tool_calls: boolean;
  streaming: boolean;
};

export type DiscoveredCatalog = {
  models: unknown[];
  cached_at: string;
  key_hash: string;
};

export type SettingsConfigV2 = {
  chat: {
    activeProviderId: string;
    activeModelId: string;
    providers: Record<string, unknown>;
    visionEnabled: boolean;
    reasoningEnabled: boolean;
    reasoningBudget: ReasoningBudget;
    capabilitiesOverride: Partial<Capabilities> | null;
  };
  image: {
    enabled: boolean;
    activeProviderId: string;
    activeModelId: string;
    providers: Record<string, unknown>;
    preset: ImagePreset;
    styleLora: string | null;
  };
  video: {
    enabled: boolean;
    activeProviderId: string;
    activeModelId: string;
    providers: Record<string, unknown>;
    mode: VideoMode;
  };
  behavior: {
    systemPrompt: string;
    temperature: number;
    uiLanguage: 'en' | 'ru';
    narrationLanguage: 'en' | 'ru';
    licenseRestrictedMode: boolean;
    agentMaxRounds: number;
    sceneTransitions: SceneTransitions;
  };
  discoveredCatalogs: Record<string, DiscoveredCatalog | null>;
};

export const DEFAULTS_V2: SettingsConfigV2 = {
  chat: {
    // Cloud chat is the generic OpenAI-compatible provider (OpenRouter
    // recommended); native Anthropic was removed in M11 Batch D.5.
    activeProviderId: 'openai-compat',
    activeModelId: 'custom',
    providers: {},
    visionEnabled: false,
    reasoningEnabled: false,
    reasoningBudget: 'medium',
    capabilitiesOverride: null,
  },
  image: {
    enabled: true,
    activeProviderId: 'local-sdxl-lightning',
    activeModelId: 'sdxl-lightning-4step',
    providers: {},
    preset: 'balanced',
    styleLora: null,
  },
  video: {
    enabled: false,
    activeProviderId: 'local-ltx-video',
    activeModelId: 'ltx-video-0.9.6-distilled',
    providers: {},
    mode: 'prerecorded',
  },
  behavior: {
    systemPrompt: 'You are a Dungeon Master assistant.',
    temperature: 0.7,
    uiLanguage: 'en',
    narrationLanguage: 'en',
    licenseRestrictedMode: false,
    agentMaxRounds: 8,
    sceneTransitions: 'auto',
  },
  discoveredCatalogs: {},
};

function isObject(x: unknown): x is Record<string, unknown> {
  return typeof x === 'object' && x !== null && !Array.isArray(x);
}

function looksLikeV2(x: Record<string, unknown>): boolean {
  return isObject(x.chat) || isObject(x.image) || isObject(x.video);
}

function canonicaliseLocalMistralrsModel(modelPath: unknown): string {
  if (typeof modelPath !== 'string') return DEFAULTS_V2.chat.activeModelId;
  // Map old underscore form (qwen3_5_4b) to dotted slug (qwen3.5-4b).
  const m = modelPath.match(/^qwen3_5_(0_8b|2b|4b|9b)$/);
  if (m?.[1]) {
    const size = m[1].replace('_', '.');
    return `qwen3.5-${size}`;
  }
  return modelPath;
}

function cloneDefaults(): SettingsConfigV2 {
  return JSON.parse(JSON.stringify(DEFAULTS_V2)) as SettingsConfigV2;
}

export type MigrateResult = {
  config: SettingsConfigV2;
  didReset: boolean;
};

export function migrateLegacySettings(raw: unknown): MigrateResult {
  if (raw === null || raw === undefined) {
    return { config: cloneDefaults(), didReset: false };
  }
  if (!isObject(raw)) {
    return { config: cloneDefaults(), didReset: true };
  }
  try {
    if (looksLikeV2(raw)) {
      const cfg = cloneDefaults();
      const partial = raw as Partial<SettingsConfigV2>;
      if (isObject(partial.chat)) Object.assign(cfg.chat, partial.chat);
      // Sanitise a legacy `anthropic` chat provider that survived a v2 blob -
      // native Anthropic was removed in M11 Batch D.5, so it falls back to the
      // openai-compat default (mirrors the v1 path below). Without this a
      // stale 'anthropic' would reach POST /settings/v2 and 400.
      if ((cfg.chat.activeProviderId as string) === 'anthropic') {
        cfg.chat.activeProviderId = DEFAULTS_V2.chat.activeProviderId;
        cfg.chat.activeModelId = DEFAULTS_V2.chat.activeModelId;
      }
      if (isObject(partial.image)) Object.assign(cfg.image, partial.image);
      if (isObject(partial.video)) Object.assign(cfg.video, partial.video);
      if (isObject(partial.behavior)) Object.assign(cfg.behavior, partial.behavior);
      if (isObject(partial.discoveredCatalogs)) {
        cfg.discoveredCatalogs = partial.discoveredCatalogs as Record<
          string,
          DiscoveredCatalog | null
        >;
      }
      return { config: cfg, didReset: false };
    }
    // v1 shape
    const v1 = raw as {
      activeProvider?: string;
      providers?: Record<string, Record<string, unknown> | undefined>;
      replicateApiKey?: string;
    };
    const cfg = cloneDefaults();
    const ap = v1.activeProvider;
    // 'anthropic' is intentionally NOT accepted: native Anthropic was removed
    // in M11 Batch D.5, so a legacy anthropic v1 falls through to the
    // openai-compat default (the user reconfigures cloud via Settings).
    if (ap === 'openai-compat' || ap === 'local-mistralrs') {
      cfg.chat.activeProviderId = ap;
    }
    if (ap === 'openai-compat') {
      const p = v1.providers?.['openai-compat'];
      if (p && typeof p.model === 'string') cfg.chat.activeModelId = p.model;
      else cfg.chat.activeModelId = 'custom';
    } else if (ap === 'local-mistralrs') {
      const p = v1.providers?.['local-mistralrs'];
      cfg.chat.activeModelId = canonicaliseLocalMistralrsModel(p?.modelPath);
    }
    if (typeof v1.replicateApiKey === 'string' && v1.replicateApiKey.length > 0) {
      cfg.image.activeProviderId = 'replicate';
      cfg.image.activeModelId = 'stability-ai/sdxl';
      cfg.image.preset = 'cloud';
      cfg.image.providers = {
        replicate: { api_key: v1.replicateApiKey, model: 'stability-ai/sdxl' },
      };
    }
    return { config: cfg, didReset: false };
  } catch {
    return { config: cloneDefaults(), didReset: true };
  }
}
