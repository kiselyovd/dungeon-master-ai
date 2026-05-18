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
    id: 'anthropic',
    display_name: 'Anthropic Claude',
    curated_models: [
      {
        model_id: 'claude-opus-4-7',
        display_name: 'Claude Opus 4.7',
        capabilities: CAPS_ALL,
        default: false,
      },
      {
        model_id: 'claude-sonnet-4-6',
        display_name: 'Claude Sonnet 4.6',
        capabilities: CAPS_ALL,
        default: false,
      },
      {
        model_id: 'claude-haiku-4-5-20251001',
        display_name: 'Claude Haiku 4.5',
        capabilities: CAPS_ALL,
        default: true,
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
