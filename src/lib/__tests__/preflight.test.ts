import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SettingsData } from '../../state/settings';
import { DISMISS_TTL_MS, dismissPreflight, isPreflightDismissed, runPreflight } from '../preflight';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeSettings(overrides: Partial<SettingsData> = {}): SettingsData {
  return {
    activeProvider: 'anthropic',
    providers: {
      anthropic: { kind: 'anthropic', apiKey: 'sk-test', model: 'claude-3-5-sonnet-20241022' },
      'openai-compat': null,
      'local-mistralrs': null,
    },
    uiLanguage: 'en',
    narrationLanguage: 'en',
    systemPrompt: '',
    temperature: 0.7,
    replicateApiKey: null,
    chatPanelWidth: 480,
    sceneTransitionsEnabled: true,
    imageEnabled: false,
    imagePreset: 'balanced',
    imageStyleLora: null,
    videoEnabled: false,
    videoMode: 'prerecorded',
    visionEnabled: false,
    reasoningEnabled: false,
    reasoningBudget: 'medium',
    licenseRestrictedMode: false,
    agentMaxRounds: 8,
    discoveredCatalogs: {},
    ...overrides,
  } as SettingsData;
}

// ---------------------------------------------------------------------------
// 1. runPreflight returns 'ok' for fully configured settings
// ---------------------------------------------------------------------------

describe('runPreflight', () => {
  it('returns ok for fully configured settings (chat configured, image+video off)', () => {
    const s = makeSettings();
    expect(runPreflight(s)).toBe('ok');
  });

  // 2. missing_chat when active provider config slot is null (not local-mistralrs)
  it('returns missing_chat when active provider has null config (not local-mistralrs)', () => {
    const s = makeSettings({
      activeProvider: 'anthropic',
      providers: {
        anthropic: null,
        'openai-compat': null,
        'local-mistralrs': null,
      },
    });
    expect(runPreflight(s)).toBe('missing_chat');
  });

  // 3. D8 tolerance: local-mistralrs with null slot is OK
  it('returns ok (not missing_chat) for activeProvider local-mistralrs with null slot', () => {
    const s = makeSettings({
      activeProvider: 'local-mistralrs',
      providers: {
        anthropic: null,
        'openai-compat': null,
        'local-mistralrs': null,
      },
    });
    expect(runPreflight(s)).toBe('ok');
  });

  // 4a. missing_image: imageEnabled + cloud preset + no Replicate key
  it('returns missing_image when imageEnabled, imagePreset cloud, no replicateApiKey', () => {
    const s = makeSettings({
      imageEnabled: true,
      imagePreset: 'cloud',
      replicateApiKey: null,
    });
    expect(runPreflight(s)).toBe('missing_image');
  });

  // 4b. missing_video: videoEnabled + live mode + no Replicate key
  it('returns missing_video when videoEnabled, videoMode live, no replicateApiKey', () => {
    const s = makeSettings({
      videoEnabled: true,
      videoMode: 'live',
      replicateApiKey: null,
    });
    expect(runPreflight(s)).toBe('missing_video');
  });

  // local presets count as configured even with no replicate key
  it('returns ok for imageEnabled with local preset even without replicateApiKey', () => {
    const s = makeSettings({
      imageEnabled: true,
      imagePreset: 'balanced',
      replicateApiKey: null,
    });
    expect(runPreflight(s)).toBe('ok');
  });

  // empty string replicateApiKey is also treated as missing
  it('returns missing_image for empty string replicateApiKey with cloud preset', () => {
    const s = makeSettings({
      imageEnabled: true,
      imagePreset: 'cloud',
      replicateApiKey: '',
    });
    expect(runPreflight(s)).toBe('missing_image');
  });

  // chat is checked before image (priority order)
  it('returns missing_chat before missing_image (priority order)', () => {
    const s = makeSettings({
      activeProvider: 'anthropic',
      providers: { anthropic: null, 'openai-compat': null, 'local-mistralrs': null },
      imageEnabled: true,
      imagePreset: 'cloud',
      replicateApiKey: null,
    });
    expect(runPreflight(s)).toBe('missing_chat');
  });
});

// ---------------------------------------------------------------------------
// 5. dismissPreflight + isPreflightDismissed
// ---------------------------------------------------------------------------

describe('dismissPreflight / isPreflightDismissed', () => {
  const STORAGE_KEY = 'dm-preflight-dismissed';

  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    localStorage.clear();
    vi.useRealTimers();
  });

  it('isPreflightDismissed(missing_chat) is always false', () => {
    // even after a dismiss attempt
    dismissPreflight('missing_chat', true);
    expect(isPreflightDismissed('missing_chat')).toBe(false);
  });

  it('dismissPreflight(missing_image, true) then isPreflightDismissed(missing_image) is true', () => {
    const now = 1_000_000;
    vi.setSystemTime(now);
    dismissPreflight('missing_image', true);
    expect(isPreflightDismissed('missing_image')).toBe(true);
  });

  it('isPreflightDismissed returns false after the 24h window expires', () => {
    const now = 1_000_000;
    vi.setSystemTime(now);
    dismissPreflight('missing_image', true);
    // advance past the TTL
    vi.setSystemTime(now + DISMISS_TTL_MS + 60 * 60 * 1000);
    expect(isPreflightDismissed('missing_image')).toBe(false);
  });

  it('isPreflightDismissed returns false for missing_video if missing_image was dismissed', () => {
    vi.setSystemTime(1_000_000);
    dismissPreflight('missing_image', true);
    expect(isPreflightDismissed('missing_video')).toBe(false);
  });

  it('is defensive about malformed localStorage JSON', () => {
    localStorage.setItem(STORAGE_KEY, '{not valid json}');
    expect(isPreflightDismissed('missing_image')).toBe(false);
  });

  // Issue 5: dismiss(false) must not persist to localStorage
  it('dismissPreflight with dontAskAgain=false does not write to localStorage', () => {
    vi.setSystemTime(1_000_000);
    dismissPreflight('missing_image', false);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
    expect(isPreflightDismissed('missing_image')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 6. runPreflight priority: image has priority over video
// ---------------------------------------------------------------------------

describe('runPreflight priority', () => {
  it('returns missing_image when both image(cloud+no key) and video(live+no key) are failing', () => {
    const s = makeSettings({
      imageEnabled: true,
      imagePreset: 'cloud',
      videoEnabled: true,
      videoMode: 'live',
      replicateApiKey: null,
    });
    expect(runPreflight(s)).toBe('missing_image');
  });
});
