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
    activeProviderId: 'anthropic',
    activeModelId: 'claude-haiku-4-5-20251001',
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
  if (m && m[1]) {
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
    if (ap === 'anthropic' || ap === 'openai-compat' || ap === 'local-mistralrs') {
      cfg.chat.activeProviderId = ap;
    }
    if (ap === 'anthropic') {
      const p = v1.providers?.anthropic;
      if (p && typeof p.model === 'string') cfg.chat.activeModelId = p.model;
    } else if (ap === 'openai-compat') {
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
