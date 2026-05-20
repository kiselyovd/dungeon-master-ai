/**
 * PresetStep tests - E2
 *
 * TDD: written before the full PresetStep implementation.
 * Covers:
 *   1. All 5 preset cards render.
 *   2. The local-only card shows the "Recommended" badge.
 *   3. Selecting cloud-cinematic then Continue calls onNext after onPresetChange.
 *   4. Local-only download-size hint renders on its card.
 */

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { PresetStep } from '../steps/PresetStep';

function setup(presetOverride?: Parameters<typeof PresetStep>[0]['preset']) {
  const onPresetChange = vi.fn();
  const onBack = vi.fn();
  const onNext = vi.fn();
  const utils = render(
    <PresetStep
      titleId="test-title"
      preset={presetOverride ?? 'local-only'}
      onPresetChange={onPresetChange}
      onBack={onBack}
      onNext={onNext}
    />,
  );
  return { ...utils, onPresetChange, onBack, onNext };
}

describe('PresetStep', () => {
  // ------------------------------------------------------------------
  // Test 1: all 5 preset cards render
  // ------------------------------------------------------------------
  it('renders all 5 preset cards', () => {
    setup();
    const group = screen.getByRole('radiogroup');
    const cards = within(group).getAllByRole('radio');
    expect(cards).toHaveLength(5);
  });

  // ------------------------------------------------------------------
  // Test 2: local-only card shows the Recommended badge
  // ------------------------------------------------------------------
  it('shows the Recommended badge on the local-only card', () => {
    setup('local-only');
    // The badge text comes from the preset_recommended i18n key -> "Recommended"
    expect(screen.getByText(/Recommended/i)).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Test 3: selecting cloud-cinematic then Continue calls onPresetChange + onNext
  // ------------------------------------------------------------------
  it('selecting cloud-cinematic then Continue calls onPresetChange and onNext', async () => {
    const user = userEvent.setup();
    const { onPresetChange, onNext } = setup('local-only');

    // Find the cloud-cinematic card by its accessible name
    const group = screen.getByRole('radiogroup');
    const cloudCard = within(group).getByRole('radio', { name: /Cloud cinematic/i });
    await user.click(cloudCard);
    expect(onPresetChange).toHaveBeenCalledWith('cloud-cinematic');

    await user.click(screen.getByRole('button', { name: /Continue/i }));
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  // ------------------------------------------------------------------
  // Test 4: local-only card shows the download-size hint
  // ------------------------------------------------------------------
  it('shows the download-size hint on the local-only card', () => {
    setup('local-only');
    // Download-size hint text: "~6.5 GB download"
    expect(screen.getByText(/6\.5 GB/i)).toBeInTheDocument();
  });
});
