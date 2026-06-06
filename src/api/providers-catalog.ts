import type { Capabilities } from '../state/settingsMigration';

export interface CuratedModel {
  model_id: string;
  display_name: string;
  capabilities: Capabilities;
  default: boolean;
}

export interface ProviderCatalogEntry {
  id: string;
  display_name: string;
  curated_models: CuratedModel[];
}

const CAPS_ALL: Capabilities = {
  vision_input: true,
  reasoning: true,
  tool_calls: true,
  streaming: true,
};

const CAPS_TEXT_WITH_TOOLS: Capabilities = {
  vision_input: false,
  reasoning: false,
  tool_calls: true,
  streaming: true,
};

export interface MediaCatalogEntry {
  id: string;
  display_name: string;
  license: string;
  preset_for_image?: 'fast' | 'balanced' | 'quality' | 'quality-oss' | 'cloud';
}

export const IMAGE_CATALOG: MediaCatalogEntry[] = [
  {
    id: 'local-sdxl-lightning',
    display_name: 'Local: SDXL-Lightning (Balanced)',
    license: 'Apache 2.0',
    preset_for_image: 'balanced',
  },
  {
    id: 'local-sdxl-turbo',
    display_name: 'Local: SDXL-Turbo (Fast)',
    license: 'SAI NC',
    preset_for_image: 'fast',
  },
  {
    id: 'local-nunchaku-flux',
    display_name: 'Local: Nunchaku FLUX (Quality)',
    license: 'FLUX-dev NC',
    preset_for_image: 'quality',
  },
  {
    id: 'local-z-image-turbo',
    display_name: 'Local: Z-Image-Turbo (Quality-OSS)',
    license: 'Apache 2.0',
    preset_for_image: 'quality-oss',
  },
  {
    id: 'replicate',
    display_name: 'Replicate (cloud)',
    license: 'varies per model',
    preset_for_image: 'cloud',
  },
];

export const VIDEO_CATALOG: MediaCatalogEntry[] = [
  {
    id: 'local-ltx-video',
    display_name: 'Local: LTX-Video 0.9.6 distilled',
    license: 'LTX (re-check before GA)',
  },
];

/**
 * Map a frontend ImagePreset literal to its catalog entry. Returns undefined
 * if no match (defensive against future preset literals not in catalog yet).
 */
export function imageEntryForPreset(preset: string): MediaCatalogEntry | undefined {
  return IMAGE_CATALOG.find((e) => e.preset_for_image === preset);
}

export const CHAT_CATALOG: ProviderCatalogEntry[] = [
  {
    id: 'local-mistralrs',
    display_name: 'Local: mistralrs',
    curated_models: [
      {
        model_id: 'qwen3.5-0.8b',
        display_name: 'Qwen3.5-0.8B',
        capabilities: CAPS_ALL,
        default: false,
      },
      {
        model_id: 'qwen3.5-2b',
        display_name: 'Qwen3.5-2B',
        capabilities: CAPS_ALL,
        default: false,
      },
      { model_id: 'qwen3.5-4b', display_name: 'Qwen3.5-4B', capabilities: CAPS_ALL, default: true },
      {
        model_id: 'qwen3.5-9b',
        display_name: 'Qwen3.5-9B',
        capabilities: CAPS_ALL,
        default: false,
      },
    ],
  },
  {
    id: 'openai-compat',
    display_name: 'OpenAI-compatible (custom)',
    curated_models: [
      {
        model_id: 'custom',
        display_name: 'Custom',
        capabilities: CAPS_TEXT_WITH_TOOLS,
        default: true,
      },
    ],
  },
];
