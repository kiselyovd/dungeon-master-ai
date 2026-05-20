import type { ProviderKind } from '../state/providers';
import type { ProvidersMap, SettingsData } from '../state/settings';
import type { ImagePreset, VideoMode } from '../state/settingsMigration';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type PreflightStatus = 'ok' | 'missing_chat' | 'missing_image' | 'missing_video';

/**
 * The subset of SettingsData fields that runPreflight reads.
 * Accepting this narrower type lets callers pass a pick of the store
 * without a type-unsafe cast.
 */
export interface PreflightInput {
  activeProvider: ProviderKind;
  providers: ProvidersMap;
  imageEnabled: boolean;
  imagePreset: ImagePreset;
  replicateApiKey: string | null;
  videoEnabled: boolean;
  videoMode: VideoMode;
}

// ---------------------------------------------------------------------------
// Pure check
// ---------------------------------------------------------------------------

/**
 * Returns the first configuration problem found, in priority order
 * chat -> image -> video, or 'ok' when everything looks good.
 *
 * D8 tolerance: activeProvider === 'local-mistralrs' with a null config slot
 * is intentional (the Local LLM tab derives its config from the localMode
 * slice), so it is never flagged as missing_chat.
 */
export function runPreflight(settings: PreflightInput | SettingsData): PreflightStatus {
  const {
    activeProvider,
    providers,
    imageEnabled,
    imagePreset,
    replicateApiKey,
    videoEnabled,
    videoMode,
  } = settings;

  // 1. Chat
  if (activeProvider !== 'local-mistralrs' && providers[activeProvider] == null) {
    return 'missing_chat';
  }

  // 2. Image - only when image is enabled and the cloud preset requires a key
  if (imageEnabled && imagePreset === 'cloud' && !replicateApiKey) {
    return 'missing_image';
  }

  // 3. Video - only when video is enabled and live mode requires a key
  if (videoEnabled && videoMode === 'live' && !replicateApiKey) {
    return 'missing_video';
  }

  return 'ok';
}

// ---------------------------------------------------------------------------
// Dismissal cache (localStorage)
// ---------------------------------------------------------------------------

/** How long a "don't ask again" dismissal is honoured. */
export const DISMISS_TTL_MS = 24 * 60 * 60 * 1000; // 24 hours

const STORAGE_KEY = 'dm-preflight-dismissed';

interface DismissRecord {
  status: PreflightStatus;
  until: number;
}

/**
 * 'missing_chat' is blocking and cannot be dismissed.
 * For image/video: if dontAskAgain is true, persist a 24h cache entry.
 * If false, we don't write anything - the modal will reappear on the next
 * session start (acceptable per spec).
 */
export function dismissPreflight(status: PreflightStatus, dontAskAgain: boolean): void {
  if (status === 'ok' || status === 'missing_chat') return;
  if (!dontAskAgain) return;

  const record: DismissRecord = {
    status,
    until: Date.now() + DISMISS_TTL_MS,
  };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(record));
  } catch {
    // localStorage may be unavailable; silently ignore
  }
}

/**
 * 'missing_chat' is always non-dismissible (returns false).
 * For image/video: returns true iff there is a valid, unexpired cache entry
 * for the given status.
 */
export function isPreflightDismissed(status: PreflightStatus): boolean {
  if (status === 'ok' || status === 'missing_chat') return false;

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return false;
    const record = JSON.parse(raw) as unknown;
    if (
      record == null ||
      typeof record !== 'object' ||
      !('status' in record) ||
      !('until' in record)
    ) {
      return false;
    }
    const rec = record as DismissRecord;
    return rec.status === status && rec.until > Date.now();
  } catch {
    return false;
  }
}
