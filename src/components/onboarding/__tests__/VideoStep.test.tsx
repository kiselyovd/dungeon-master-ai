/**
 * VideoStep tests - E5
 *
 * TDD: written before the full VideoStep implementation.
 * Covers:
 *   1. With replicateApiKey present: video_key_reused copy shows; Enable button is enabled;
 *      clicking Enable sets videoEnabled=true + videoMode='live', calls postSettingsV2, calls onNext.
 *   2. With no replicateApiKey: Enable button is disabled; video_key_missing_hint copy shows;
 *      clicking Skip sets videoEnabled=false, calls postSettingsV2, calls onNext.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { useStore } from '../../../state/useStore';
import { VideoStep } from '../steps/VideoStep';

// ---------------------------------------------------------------------------
// Module-level mocks
// ---------------------------------------------------------------------------

vi.mock('../../../api/settings', () => ({
  postSettingsV2: vi.fn(async () => undefined),
}));

vi.mock('../../../api/client', () => ({
  backendUrl: vi.fn(async (path: string) => `http://localhost:3000${path}`),
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

import { postSettingsV2 } from '../../../api/settings';

const mockedPostSettings = vi.mocked(postSettingsV2);

function setup() {
  const onBack = vi.fn();
  const onNext = vi.fn();
  const utils = render(<VideoStep titleId="test-title" onBack={onBack} onNext={onNext} />);
  return { ...utils, onBack, onNext };
}

// Reset store and mocks between tests.
beforeEach(() => {
  vi.clearAllMocks();
  useStore.getState().settings.setVideoEnabled(false);
  useStore.getState().settings.setVideoMode('prerecorded');
  useStore.getState().settings.setReplicateApiKey(null);
});

// ---------------------------------------------------------------------------
// Test 1: with replicateApiKey present
// ---------------------------------------------------------------------------
describe('VideoStep - with Replicate API key', () => {
  it('shows key-reused confirmation; Enable button enabled; clicking Enable sets videoEnabled=true + videoMode=live + postSettingsV2 + onNext', async () => {
    useStore.getState().settings.setReplicateApiKey('r8_test1234567890');

    const user = userEvent.setup();
    const { onNext } = setup();

    // video_key_reused copy must be visible
    expect(
      await screen.findByText(/replicate api key from the previous step/i),
    ).toBeInTheDocument();

    // Enable button must be present and enabled
    const enableBtn = screen.getByRole('button', { name: /enable/i });
    expect(enableBtn).not.toBeDisabled();

    await user.click(enableBtn);

    // Store state must be updated
    const state = useStore.getState().settings;
    expect(state.videoEnabled).toBe(true);
    expect(state.videoMode).toBe('live');

    // postSettingsV2 must have been called
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must advance
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 2: with NO replicateApiKey
// ---------------------------------------------------------------------------
describe('VideoStep - without Replicate API key', () => {
  it('Enable button disabled; missing-hint copy shows; Skip sets videoEnabled=false + postSettingsV2 + onNext', async () => {
    // replicateApiKey is null from beforeEach reset
    const user = userEvent.setup();
    const { onNext } = setup();

    // video_key_missing_hint copy must be visible
    expect(await screen.findByText(/replicate api key is needed/i)).toBeInTheDocument();

    // Enable button must be disabled
    const enableBtn = screen.getByRole('button', { name: /enable/i });
    expect(enableBtn).toBeDisabled();

    // Skip button must be present and enabled - click it
    const skipBtn = screen.getByRole('button', { name: /skip/i });
    await user.click(skipBtn);

    // Store state must reflect disabled video
    const state = useStore.getState().settings;
    expect(state.videoEnabled).toBe(false);

    // postSettingsV2 must have been called
    expect(mockedPostSettings).toHaveBeenCalledTimes(1);

    // onNext must advance
    await waitFor(() => {
      expect(onNext).toHaveBeenCalledTimes(1);
    });
  });
});

// ---------------------------------------------------------------------------
// Test 3: postSettingsV2 rejection surfaces error inline; onNext not called
// ---------------------------------------------------------------------------
describe('VideoStep - postSettingsV2 rejection', () => {
  it('shows inline error and does not call onNext when Enable triggers a network failure', async () => {
    useStore.getState().settings.setReplicateApiKey('r8_test1234567890');
    mockedPostSettings.mockRejectedValueOnce(new Error('Network timeout'));

    const user = userEvent.setup();
    const { onNext } = setup();

    // Enable button must be enabled (key is present)
    const enableBtn = await screen.findByRole('button', { name: /enable/i });
    expect(enableBtn).not.toBeDisabled();

    await user.click(enableBtn);

    // Inline error must appear
    await waitFor(() => {
      expect(screen.getByText(/network timeout/i)).toBeInTheDocument();
    });

    // onNext must NOT have been called
    expect(onNext).not.toHaveBeenCalled();
  });
});
