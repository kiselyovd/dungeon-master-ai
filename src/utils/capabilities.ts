import { CHAT_CATALOG } from '../api/providers-catalog';
import type { ProviderKind } from '../state/providers';
import type { Capabilities } from '../state/settingsMigration';

const FALLBACK_CAPS: Capabilities = {
  vision_input: false,
  reasoning: false,
  tool_calls: false,
  streaming: false,
};

/**
 * Lookup the catalog-declared capabilities for the active provider + model.
 * Used by SettingsForm to gate UI toggles (vision, reasoning) on what the
 * model actually supports. Falls back to all-false if not in catalog.
 */
export function activeProviderCaps(providerId: ProviderKind, modelId: string): Capabilities {
  const entry = CHAT_CATALOG.find((e) => e.id === providerId);
  if (!entry) return FALLBACK_CAPS;
  const model = entry.curated_models.find((m) => m.model_id === modelId);
  if (model) return model.capabilities;
  // No exact model match (custom or discovered) - return the first curated model's caps as a baseline
  return entry.curated_models[0]?.capabilities ?? FALLBACK_CAPS;
}
