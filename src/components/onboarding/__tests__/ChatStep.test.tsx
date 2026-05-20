/**
 * ChatStep tests - E3
 *
 * TDD: written before the full ChatStep implementation.
 * Covers:
 *   1. Local-only: renders download button; clicking starts download (mocked useModelDownload).
 *   2. Local-only: model already installed shows "Model ready" + Continue enabled.
 *   2b. Local-only: Continue persists local-mistralrs provider (setProviderConfig + setActiveProvider + postSettingsV2).
 *   3. Cloud/Hybrid: empty key blocks Continue; entering key + Continue persists provider.
 *   4. Text-only: Skip calls onNext without persisting provider.
 *   5. Manual: auto-advances on mount, calls onNext exactly once (StrictMode-safe).
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { ChatStep } from '../steps/ChatStep';

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

// backendUrl is used by fetchLocalLlmManifest inside the mock, but since we
// mock the whole module we don't need it - mock it anyway to silence imports.
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

function setup(preset: Parameters<typeof ChatStep>[0]['preset']) {
  const onBack = vi.fn();
  const onNext = vi.fn();
  const utils = render(
    <ChatStep titleId="test-title" preset={preset} onBack={onBack} onNext={onNext} />,
  );
  return { ...utils, onBack, onNext };
}

// Reset store and mocks between tests.
beforeEach(() => {
  vi.clearAllMocks();
  // Reset downloads to idle.
  useStore.getState().localMode.setDownloadState('qwen3_5_4b', { state: 'idle' });
});

// ---------------------------------------------------------------------------
// Test 1: local-only - renders download button; clicking starts download
// ---------------------------------------------------------------------------
describe('ChatStep - local-only preset', () => {
  it('renders download button and starts download on click', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: [],
      system: [],
      user: [],
      download_states: {},
    });

    const user = userEvent.setup();
    setup('local-only');

    // Download button must be visible.
    const downloadBtn = await screen.findByRole('button', { name: /download/i });
    expect(downloadBtn).toBeInTheDocument();

    // Simulate download in progress after click.
    mockStart.mockImplementationOnce(() => {
      useStore.getState().localMode.setDownloadState('qwen3_5_4b', {
        state: 'downloading',
        bytesDone: 0,
        totalBytes: 2_500_000_000,
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
  // Test 2: local-only - model already installed -> "Model ready" + Continue enabled
  // -------------------------------------------------------------------------
  it('shows model-ready state and enables Continue when model is already installed', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: ['qwen3.5-4b'],
      system: [],
      user: [],
      download_states: {},
    });

    const { onNext } = setup('local-only');

    // "Model ready" text visible (from i18n key chat_local_already_exists).
    await waitFor(() => {
      expect(screen.getByText(/model ready|already installed|ready/i)).toBeInTheDocument();
    });

    // Continue button is enabled.
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    expect(continueBtn).not.toBeDisabled();

    // Clicking Continue calls onNext.
    const user = userEvent.setup();
    await user.click(continueBtn);
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  // -------------------------------------------------------------------------
  // Test 2b: local-only Continue persists the local-mistralrs provider
  // -------------------------------------------------------------------------
  it('Continue after model already installed persists local-mistralrs provider', async () => {
    mockedFetchManifest.mockResolvedValue({
      installed_ids: ['qwen3.5-4b'],
      system: [],
      user: [],
      download_states: {},
    });

    const { onNext } = setup('local-only');

    await waitFor(() => {
      expect(screen.getByText(/model ready|already installed|ready/i)).toBeInTheDocument();
    });

    const user = userEvent.setup();
    const continueBtn = screen.getByRole('button', { name: /continue/i });
    await user.click(continueBtn);

    // Provider config must be set to local-mistralrs with correct model path.
    const state = useStore.getState().settings;
    expect(state.providers['local-mistralrs']).not.toBeNull();
    expect(state.providers['local-mistralrs']?.modelPath).toBe('qwen3_5_4b');
    expect(state.activeProvider).toBe('local-mistralrs');

    // postSettingsV2 must have been called.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must be called to advance.
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 3: cloud-cinematic - empty key blocks Continue; entering key + Continue
//         persists provider via setProviderConfig + setActiveProvider + postSettingsV2
// ---------------------------------------------------------------------------
describe('ChatStep - cloud-cinematic preset', () => {
  it('blocks Continue on empty key; entering key + Continue persists provider', async () => {
    const user = userEvent.setup();
    const { onNext } = setup('cloud-cinematic');

    const continueBtn = screen.getByRole('button', { name: /continue/i });

    // Continue is disabled initially (no key).
    expect(continueBtn).toBeDisabled();

    // Type a valid API key.
    const keyInput = screen.getByPlaceholderText(/sk-ant/i);
    await user.type(keyInput, 'sk-ant-api03-testkey123');

    // Now enabled.
    await waitFor(() => {
      expect(continueBtn).not.toBeDisabled();
    });

    await user.click(continueBtn);

    // Provider config + activeProvider must be set in the store.
    const state = useStore.getState().settings;
    expect(state.providers.anthropic).not.toBeNull();
    expect(state.providers.anthropic?.apiKey).toBe('sk-ant-api03-testkey123');
    expect(state.activeProvider).toBe('anthropic');

    // postSettingsV2 must have been called.
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must be called to advance.
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 4: text-only - Skip advances without persisting provider
// ---------------------------------------------------------------------------
describe('ChatStep - text-only preset', () => {
  it('Skip calls onNext without persisting a provider', async () => {
    const user = userEvent.setup();
    const { onNext } = setup('text-only');

    const skipBtn = screen.getByRole('button', { name: /skip/i });
    await user.click(skipBtn);

    expect(onNext).toHaveBeenCalledTimes(1);
    expect(mockedPostSettings).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Test 5: manual - auto-advances on mount exactly once (StrictMode double-mount)
// ---------------------------------------------------------------------------
describe('ChatStep - manual preset', () => {
  it('calls onNext exactly once on mount (StrictMode-safe)', async () => {
    const onBack = vi.fn();
    const onNext = vi.fn();

    render(
      <React.StrictMode>
        <ChatStep titleId="test-title" preset="manual" onBack={onBack} onNext={onNext} />
      </React.StrictMode>,
    );

    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});
