/**
 * Preset definitions and step sequence computation for the 6-step onboarding
 * state machine.
 *
 * A "preset" is a named configuration profile that determines which optional
 * steps are shown (image, video) and pre-seeds the provider and modality
 * settings after onboarding completes.
 */

export type Step = 'welcome' | 'preset' | 'chat' | 'image' | 'video' | 'hero';

export type PresetId = 'local-only' | 'cloud-cinematic' | 'hybrid' | 'text-only' | 'manual';

export interface Preset {
  id: PresetId;
  /** i18n key in the onboarding namespace for the display name */
  labelKey: string;
  /** i18n key for the short description */
  descKey: string;
  /** Whether this preset is the recommended default */
  recommended: boolean;
}

export const PRESETS: readonly Preset[] = [
  {
    id: 'local-only',
    labelKey: 'preset_local_only_name',
    descKey: 'preset_local_only_desc',
    recommended: true,
  },
  {
    id: 'cloud-cinematic',
    labelKey: 'preset_cloud_cinematic_name',
    descKey: 'preset_cloud_cinematic_desc',
    recommended: false,
  },
  {
    id: 'hybrid',
    labelKey: 'preset_hybrid_name',
    descKey: 'preset_hybrid_desc',
    recommended: false,
  },
  {
    id: 'text-only',
    labelKey: 'preset_text_only_name',
    descKey: 'preset_text_only_desc',
    recommended: false,
  },
  {
    id: 'manual',
    labelKey: 'preset_manual_name',
    descKey: 'preset_manual_desc',
    recommended: false,
  },
] as const;

/** The default preset used before a preset is explicitly chosen. */
export const DEFAULT_PRESET: PresetId = 'local-only';

/**
 * Returns the ordered step sequence for the given preset.
 *
 * Rules:
 * - Always included: welcome, preset, chat, hero
 * - image step: skipped when preset is 'text-only'
 * - video step: only shown for 'cloud-cinematic'
 *
 * Examples:
 *   computeSteps('local-only')       -> ['welcome','preset','chat','image','hero']
 *   computeSteps('cloud-cinematic')  -> ['welcome','preset','chat','image','video','hero']
 *   computeSteps('text-only')        -> ['welcome','preset','chat','hero']
 *   computeSteps('hybrid')           -> ['welcome','preset','chat','image','hero']
 *   computeSteps('manual')           -> ['welcome','preset','chat','image','hero']
 */
export function computeSteps(preset: PresetId): Step[] {
  const steps: Step[] = ['welcome', 'preset', 'chat'];
  if (preset !== 'text-only') {
    steps.push('image');
  }
  if (preset === 'cloud-cinematic') {
    steps.push('video');
  }
  steps.push('hero');
  return steps;
}
