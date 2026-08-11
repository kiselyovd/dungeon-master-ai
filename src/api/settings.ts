/**
 * HTTP client for POST /settings/v2.
 *
 * The endpoint speaks the nested snake_case wire shape (SettingsConfigV2 in
 * crates/app-server/src/routes/settings/v2.rs). Frontend SettingsData is flat
 * camelCase. This module is the only place that translates between them.
 */

import { backendUrl } from './client';
import { ChatError } from './errors';

export type ImagePreset = 'fast' | 'balanced' | 'quality' | 'quality-oss' | 'cloud';
export type ProviderConfig =
  | { kind: 'openai-compat'; baseUrl: string; apiKey: string; model: string }
  | { kind: 'local-mistralrs'; modelPath: string; contextWindow: number };

export interface SettingsInput {
  activeProvider: 'openai-compat' | 'local-mistralrs';
  providers: Record<'openai-compat' | 'local-mistralrs', ProviderConfig | null>;
  uiLanguage: 'en' | 'ru';
  narrationLanguage: 'en' | 'ru';
  systemPrompt: string;
  temperature: number;
  replicateApiKey: string | null;
  sceneTransitionsEnabled: boolean;
  imageEnabled: boolean;
  imagePreset: ImagePreset;
  imageStyleLora: string | null;
  videoEnabled: boolean;
  videoMode: 'prerecorded' | 'live' | 'race';
  visionEnabled: boolean;
  reasoningEnabled: boolean;
  reasoningBudget: 'low' | 'medium' | 'high';
  licenseRestrictedMode: boolean;
  agentMaxRounds: number;
}

export interface LocalRuntimeSettingsInput {
  runtime: { llm: { state: string; port?: number } };
  customLlmOverride: { hf_repo?: string } | null;
}

interface BackendError {
  error?: { code?: string; message?: string };
}

const PRESET_MAP: Record<ImagePreset, { providerId: string; modelId: string }> = {
  fast: { providerId: 'local-sdxl-turbo', modelId: 'sdxl-turbo-fp16' },
  balanced: { providerId: 'local-sdxl-lightning', modelId: 'sdxl-lightning-4step' },
  quality: { providerId: 'local-nunchaku-flux', modelId: 'flux-dev-int4-turbo-alpha-8step' },
  'quality-oss': { providerId: 'local-z-image-turbo', modelId: 'z-image-turbo-svdq-int4' },
  cloud: { providerId: 'replicate', modelId: 'stability-ai/sdxl' },
};

const VIDEO_DEFAULTS = {
  providerId: 'local-ltx-video',
  modelId: 'ltx-video-0.9.6-distilled',
};

interface ChatWireSlice {
  active_provider_id: string;
  active_model_id: string;
  providers: Record<string, unknown>;
  vision_enabled: boolean;
  reasoning_enabled: boolean;
  reasoning_budget: 'low' | 'medium' | 'high';
  capabilities_override: null;
}

interface ImageWireSlice {
  enabled: boolean;
  active_provider_id: string;
  active_model_id: string;
  providers: Record<string, unknown>;
  preset: ImagePreset;
  style_lora: string | null;
}

interface VideoWireSlice {
  enabled: boolean;
  active_provider_id: string;
  active_model_id: string;
  providers: Record<string, unknown>;
  mode: 'prerecorded' | 'live' | 'race';
}

interface BehaviorWireSlice {
  system_prompt: string;
  temperature: number;
  ui_language: string;
  narration_language: string;
  license_restricted_mode: boolean;
  agent_max_rounds: number;
  scene_transitions: 'auto' | 'manual' | 'off';
}

interface V2Wire {
  chat: ChatWireSlice;
  image: ImageWireSlice;
  video: VideoWireSlice;
  behavior: BehaviorWireSlice;
}

export function toV2Wire(
  settings: SettingsInput,
  localRuntime?: LocalRuntimeSettingsInput,
): V2Wire {
  const activeProviderId = settings.activeProvider;
  const activeProviderConfig = settings.providers[activeProviderId];
  if (!activeProviderConfig) {
    throw new ChatError(
      'provider_error',
      `Active provider "${activeProviderId}" has no configured slice.`,
    );
  }
  const { providersSlice, activeModelId } = buildChatProvidersSlice(
    activeProviderConfig,
    localRuntime,
  );
  const presetMapping = PRESET_MAP[settings.imagePreset];

  return {
    chat: {
      active_provider_id: activeProviderId,
      active_model_id: activeModelId,
      providers: providersSlice,
      vision_enabled: settings.visionEnabled,
      reasoning_enabled: settings.reasoningEnabled,
      reasoning_budget: settings.reasoningBudget,
      capabilities_override: null,
    },
    image: {
      enabled: settings.imageEnabled,
      active_provider_id: presetMapping.providerId,
      active_model_id: presetMapping.modelId,
      providers:
        settings.imagePreset === 'cloud' && settings.replicateApiKey
          ? { replicate: { api_key: settings.replicateApiKey } }
          : {},
      preset: settings.imagePreset,
      style_lora: settings.imageStyleLora,
    },
    video: {
      enabled: settings.videoEnabled,
      active_provider_id: VIDEO_DEFAULTS.providerId,
      active_model_id: VIDEO_DEFAULTS.modelId,
      providers: {},
      mode: settings.videoMode,
    },
    behavior: {
      system_prompt: settings.systemPrompt,
      temperature: settings.temperature,
      ui_language: settings.uiLanguage,
      narration_language: settings.narrationLanguage,
      license_restricted_mode: settings.licenseRestrictedMode,
      agent_max_rounds: settings.agentMaxRounds,
      scene_transitions: settings.sceneTransitionsEnabled ? 'auto' : 'off',
    },
  };
}

function buildChatProvidersSlice(
  c: ProviderConfig,
  localRuntime?: LocalRuntimeSettingsInput,
): {
  providersSlice: Record<string, unknown>;
  activeModelId: string;
} {
  switch (c.kind) {
    case 'openai-compat':
      return {
        providersSlice: {
          'openai-compat': { base_url: c.baseUrl, api_key: c.apiKey },
        },
        activeModelId: c.model,
      };
    case 'local-mistralrs': {
      if (
        localRuntime === undefined ||
        localRuntime.runtime.llm.state !== 'ready' ||
        localRuntime.runtime.llm.port === undefined
      ) {
        throw new ChatError(
          'provider_error',
          'local runtime is not ready - start it in Settings before saving.',
        );
      }
      const runtime = localRuntime.runtime.llm;
      const modelIdWire: unknown = localRuntime.customLlmOverride
        ? { custom: localRuntime.customLlmOverride }
        : c.modelPath;
      const modelLabel =
        localRuntime.customLlmOverride && typeof localRuntime.customLlmOverride === 'object'
          ? `custom:${localRuntime.customLlmOverride.hf_repo ?? 'hf'}`
          : c.modelPath;
      return {
        providersSlice: {
          'local-mistralrs': { model_id: modelIdWire, port: runtime.port },
        },
        activeModelId: modelLabel,
      };
    }
    default:
      return assertNeverProvider(c);
  }
}

export async function postSettingsV2(
  settings: SettingsInput,
  localRuntime?: LocalRuntimeSettingsInput,
): Promise<void> {
  const wire = toV2Wire(settings, localRuntime);
  const url = await backendUrl('/settings/v2');
  let resp: Response;
  try {
    resp = await fetch(url, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(wire),
    });
  } catch (e) {
    throw ChatError.from(e);
  }
  if (!resp.ok) {
    const parsed = (await resp.json().catch(() => ({}) as BackendError)) as BackendError;
    const message = parsed.error?.message ?? `POST /settings/v2 HTTP ${resp.status}`;
    throw new ChatError('provider_error', message);
  }
}

function assertNeverProvider(value: never): never {
  throw new Error(`unhandled provider kind: ${String(value)}`);
}
