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
  /** Icon name from the Icons object to display on the card */
  icon: 'Cpu' | 'Cloud' | 'Sparkle' | 'Book' | 'Settings';
  /** i18n key for the providers preview line (e.g. "Claude + FLUX-Pro + Kling") */
  providersKey: string;
  /** i18n key for the download-size hint, or null if no download required */
  downloadKey: string | null;
}

export const PRESETS: readonly Preset[] = [
  {
    id: 'local-only',
    labelKey: 'preset_local_only_name',
    descKey: 'preset_local_only_desc',
    recommended: true,
    icon: 'Cpu',
    providersKey: 'preset_local_only_providers',
    downloadKey: 'preset_local_only_download',
  },
  {
    id: 'cloud-cinematic',
    labelKey: 'preset_cloud_cinematic_name',
    descKey: 'preset_cloud_cinematic_desc',
    recommended: false,
    icon: 'Cloud',
    providersKey: 'preset_cloud_cinematic_providers',
    downloadKey: null,
  },
  {
    id: 'hybrid',
    labelKey: 'preset_hybrid_name',
    descKey: 'preset_hybrid_desc',
    recommended: false,
    icon: 'Sparkle',
    providersKey: 'preset_hybrid_providers',
    downloadKey: 'preset_hybrid_download',
  },
  {
    id: 'text-only',
    labelKey: 'preset_text_only_name',
    descKey: 'preset_text_only_desc',
    recommended: false,
    icon: 'Book',
    providersKey: 'preset_text_only_providers',
    downloadKey: null,
  },
  {
    id: 'manual',
    labelKey: 'preset_manual_name',
    descKey: 'preset_manual_desc',
    recommended: false,
    icon: 'Settings',
    providersKey: 'preset_manual_providers',
    downloadKey: null,
  },
] as const;

/** The default preset used before a preset is explicitly chosen. */
export const DEFAULT_PRESET: PresetId = 'local-only';

/**
 * Returns the ordered step sequence for the given preset.
 *
 * Rules:
 * - Hero comes right after the preset choice, before any technical setup: the
 *   narrative choice (who you are) precedes provider/modality wiring.
 * - 'manual' is an honest "skip setup": welcome, preset, hero only - no
 *   provider/modality steps. The user lands on an empty canvas and configures
 *   everything in Settings afterwards.
 * - chat: included for every preset except 'manual'.
 * - image: skipped for 'text-only' and 'manual'.
 * - video: only shown for 'cloud-cinematic'.
 *
 * Examples:
 *   computeSteps('local-only')       -> ['welcome','preset','hero','chat','image']
 *   computeSteps('cloud-cinematic')  -> ['welcome','preset','hero','chat','image','video']
 *   computeSteps('text-only')        -> ['welcome','preset','hero','chat']
 *   computeSteps('hybrid')           -> ['welcome','preset','hero','chat','image']
 *   computeSteps('manual')           -> ['welcome','preset','hero']
 */
export function computeSteps(preset: PresetId): Step[] {
  const steps: Step[] = ['welcome', 'preset', 'hero'];
  // 'manual' skips all technical setup - it is the explicit "I'll configure it
  // later" path, so it must not land the user on a blocking preflight modal.
  if (preset === 'manual') {
    return steps;
  }
  steps.push('chat');
  if (preset !== 'text-only') {
    steps.push('image');
  }
  if (preset === 'cloud-cinematic') {
    steps.push('video');
  }
  return steps;
}
