/**
 * ImageStep tests - E4
 *
 * TDD: written before the full ImageStep implementation.
 * Covers:
 *   1. Local-only: renders SDXL-Turbo download CTA; clicking starts download; progress UI shows.
 *   2. Local-only Continue (after completed/already-installed): calls setImageEnabled(true) +
 *      setImagePreset('fast') + postSettingsV2, then onNext. Asserts via store state.
 *   3. Cloud cinematic: empty Replicate key blocks Continue; entering a key + Continue persists
 *      replicateApiKey + imagePreset('cloud') + imageEnabled(true) via postSettingsV2, then onNext.
 *   4. Skip: clicking Skip sets imageEnabled=false and advances (onNext).
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { ImageStep } from '../steps/ImageStep';

// ---------------------------------------------------------------------------
// Module-level mocks
// ---------------------------------------------------------------------------

const mockStart = vi.fn();
const mockCancel = vi.fn();
vi.mock('../../../hooks/useModelDownload', () => ({
  useModelDownload: vi.fn(() => ({ start: mockStart, cancel: mockCancel })),
}));

vi.mock('../../../api/localLlm', () => ({
  fetchLocalLlmManifest: vi.fn(async () => ({
    installed_ids: [],
    system: [],
    user: [],
    download_states: {},
  })),
}));

vi.mock('../../../api/settings', () => ({
  postSettingsV2: vi.fn(async () => undefined),
}));

vi.mock('../../../api/client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://localhost:3000${path}`),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

import { fetchLocalLlmManifest } from '../../../api/localLlm';
import { postSettingsV2 } from '../../../api/settings';

const mockedFetchManifest = vi.mocked(fetchLocalLlmManifest);
const mockedPostSettings = vi.mocked(postSettingsV2);

function setup(preset: Parameters<typeof ImageStep>[0]['preset']) {
  const onBack = vi.fn();
  const onNext = vi.fn();
  const utils = render(
    <ImageStep titleId="test-title" preset={preset} onBack={onBack} onNext={onNext} />,
  );
  return { ...utils, onBack, onNext };
}

// Reset store and mocks between tests.
beforeEach(() => {
  vi.clearAllMocks();
  // Reset downloads to idle and image settings to defaults.
  useStore.getState().localMode.setDownloadState('sdxl_turbo', { state: 'idle' });
  useStore.getState().settings.setImageEnabled(false);
  useStore.getState().settings.setImagePreset('balanced');
  useStore.getState().settings.setReplicateApiKey(null);
});

// ---------------------------------------------------------------------------
// Test 1: local-only - renders download button; clicking starts download; progress shows
// ---------------------------------------------------------------------------
describe('ImageStep - local-only preset', () => {
  it('renders SDXL-Turbo download CTA and starts download on click; progress UI shows', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: [],
      system: [],
      user: [],
      download_states: {},
    });

    const user = userEvent.setup();
    setup('local-only');

    // Download CTA button must be visible.
    const downloadBtn = await screen.findByRole('button', { name: /download/i });
    expect(downloadBtn).toBeInTheDocument();

    // Simulate download in progress after click.
    mockStart.mockImplementationOnce(() => {
      useStore.getState().localMode.setDownloadState('sdxl_turbo', {
        state: 'downloading',
        bytesDone: 0,
        totalBytes: 7_000_000_000,
      });
      return Promise.resolve();
    });

    await user.click(downloadBtn);
    expect(mockStart).toHaveBeenCalledTimes(1);

    // Progress indicator should appear.
    await waitFor(() => {
      expect(screen.getByRole('progressbar')).toBeInTheDocument();
    });
  });

  // -------------------------------------------------------------------------
  // Test 2: local-only Continue - asserts store state + postSettingsV2 + onNext
  // -------------------------------------------------------------------------
  it('Continue after model installed sets imageEnabled=true + imagePreset=fast + postSettingsV2 + onNext', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: ['sdxl_turbo'],
      system: [],
      user: [],
      download_states: {},
    });

    const { onNext } = setup('local-only');

    // Wait for already-installed state to render.
    await waitFor(() => {
      expect(screen.getByText(/already installed|ready|model ready/i)).toBeInTheDocument();
    });

    // Continue button must be enabled.
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    expect(continueBtn).not.toBeDisabled();

    const user = userEvent.setup();
    await user.click(continueBtn);

    // Store state must be updated.
    const state = useStore.getState().settings;
    expect(state.imageEnabled).toBe(true);
    expect(state.imagePreset).toBe('fast');

    // postSettingsV2 must have been called.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must be called to advance.
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 3: cloud-cinematic - empty key blocks Continue; entering key + Continue persists
// ---------------------------------------------------------------------------
describe('ImageStep - cloud-cinematic preset', () => {
  it('blocks Continue on empty Replicate key; entering key + Continue persists cloud image settings', async () => {
    const user = userEvent.setup();
    const { onNext } = setup('cloud-cinematic');

    const continueBtn = screen.getByRole('button', { name: /continue/i });

    // Continue is disabled initially (no key).
    expect(continueBtn).toBeDisabled();

    // Type a Replicate API key.
    const keyInput = screen.getByLabelText(/replicate/i);
    await user.type(keyInput, 'fake-replicate-key');

    // Now enabled.
    await waitFor(() => {
      expect(continueBtn).not.toBeDisabled();
    });

    await user.click(continueBtn);

    // Store must be updated.
    const state = useStore.getState().settings;
    expect(state.replicateApiKey).toBe('fake-replicate-key');
    expect(state.imagePreset).toBe('cloud');
    expect(state.imageEnabled).toBe(true);

    // postSettingsV2 must have been called.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must advance.
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 3b: hybrid preset Continue - sets imagePreset='balanced'
// ---------------------------------------------------------------------------
describe('ImageStep - hybrid preset', () => {
  it('Continue after model installed sets imageEnabled=true + imagePreset=balanced + postSettingsV2 + onNext', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: ['sdxl_turbo'],
      system: [],
      user: [],
      download_states: {},
    });

    const { onNext } = setup('hybrid');

    // Wait for already-installed state to render.
    await waitFor(() => {
      expect(screen.getByText(/already installed|ready|model ready/i)).toBeInTheDocument();
    });

    // Continue button must be enabled.
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    expect(continueBtn).not.toBeDisabled();

    const user = userEvent.setup();
    await user.click(continueBtn);

    // Store state must reflect 'balanced' preset for hybrid.
    const state = useStore.getState().settings;
    expect(state.imageEnabled).toBe(true);
    expect(state.imagePreset).toBe('balanced');

    // postSettingsV2 must have been called.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must advance.
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 4: Skip - sets imageEnabled=false and advances
// ---------------------------------------------------------------------------
describe('ImageStep - skip behavior', () => {
  it('clicking Skip sets imageEnabled=false and calls onNext', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: [],
      system: [],
      user: [],
      download_states: {},
    });

    // Start with imageEnabled=true to verify skip turns it off.
    useStore.getState().settings.setImageEnabled(true);

    const user = userEvent.setup();
    const { onNext } = setup('local-only');

    // Wait for the download button to appear (ensures step is rendered).
    await screen.findByRole('button', { name: /download/i });

    const skipBtn = screen.getByRole('button', { name: /skip/i });
    await user.click(skipBtn);

    // imageEnabled must be false in the store.
    expect(useStore.getState().settings.imageEnabled).toBe(false);

    // postSettingsV2 should have been called to persist the disabled state.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must advance.
    expect(onNext).toHaveBeenCalledTimes(1);
  });
});
