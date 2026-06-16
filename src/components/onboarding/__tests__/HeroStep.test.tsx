/**
 * HeroStep tests - E6
 *
 * TDD: written before the full HeroStep implementation.
 * Covers:
 *   1. All 4 hero cards render.
 *   2. A hero card shows accurate preview stats matching the pc.ts preset.
 *   3. Clicking a hero card calls applyPreset + advances onboarding to completion.
 *   4. "Build from scratch" calls onExitToWizard and completes onboarding
 *      without calling applyPreset.
 *   5. HERO_PORTRAIT map exposes the 4 hero classes (unit assertion).
 */

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../../i18n';
import { HERO_PORTRAIT } from '../../../lib/heroPortraits';
import { useStore } from '../../../state/useStore';
import { HeroStep } from '../steps/HeroStep';

function setup(overrides: Partial<Parameters<typeof HeroStep>[0]> = {}) {
  const onBack = vi.fn();
  const onNext = vi.fn();
  const utils = render(
    <HeroStep titleId="test-title" onBack={onBack} onNext={onNext} {...overrides} />,
  );
  return { ...utils, onBack, onNext };
}

describe('HeroStep', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  // ------------------------------------------------------------------
  // Test 1: all 4 hero cards render
  // ------------------------------------------------------------------
  it('renders all 4 hero cards', () => {
    const { container } = setup();
    // Each hero card is a plain action button (immediate-action trigger, not a radio).
    const cards = container.querySelectorAll('.dm-hero-card');
    expect(cards).toHaveLength(4);
  });

  // ------------------------------------------------------------------
  // Test 2: hero card shows accurate preview stats matching pc.ts preset
  // ------------------------------------------------------------------
  it('shows accurate preview stats for the fighter card', () => {
    setup();
    // Fighter preset: Human, HP 12, AC 16
    const fighterCard = screen.getByRole('button', { name: /Fighter/i });
    expect(within(fighterCard).getByText(/Human/i)).toBeInTheDocument();
    expect(within(fighterCard).getByText('12')).toBeInTheDocument();
    expect(within(fighterCard).getByText('16')).toBeInTheDocument();
  });

  it('shows accurate preview stats for the wizard card', () => {
    setup();
    // Wizard preset: High Elf, HP 8, AC 12
    const wizardCard = screen.getByRole('button', { name: /Wizard/i });
    expect(within(wizardCard).getByText(/High Elf/i)).toBeInTheDocument();
    expect(within(wizardCard).getByText('8')).toBeInTheDocument();
    expect(within(wizardCard).getByText('12')).toBeInTheDocument();
  });

  // ------------------------------------------------------------------
  // Test 3: clicking a hero card calls applyPreset + completes onboarding
  // ------------------------------------------------------------------
  it('clicking the fighter card calls applyPreset and triggers onNext finalize', async () => {
    const user = userEvent.setup();
    const applyPreset = vi.spyOn(useStore.getState().pc, 'applyPreset');
    const { onNext } = setup();

    const fighterCard = screen.getByRole('button', { name: /Fighter/i });
    await user.click(fighterCard);

    expect(applyPreset).toHaveBeenCalledWith('fighter');
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  it('clicking the rogue card calls applyPreset("rogue") and triggers onNext', async () => {
    const user = userEvent.setup();
    const applyPreset = vi.spyOn(useStore.getState().pc, 'applyPreset');
    const { onNext } = setup();

    const rogueCard = screen.getByRole('button', { name: /Rogue/i });
    await user.click(rogueCard);

    expect(applyPreset).toHaveBeenCalledWith('rogue');
    expect(onNext).toHaveBeenCalledTimes(1);
  });

  // ------------------------------------------------------------------
  // Test 4: "Build from scratch" advances WITHOUT applying a preset. The
  //         full wizard is opened later by Onboarding.finalize (when no hero
  //         class was chosen), not synchronously here - so hero can sit before
  //         the chat/image steps without the wizard overlapping onboarding.
  // ------------------------------------------------------------------
  it('"Build from scratch" advances via onNext without applyPreset', async () => {
    const user = userEvent.setup();
    const applyPreset = vi.spyOn(useStore.getState().pc, 'applyPreset').mockClear();
    const { onNext } = setup();

    const scratchBtn = screen.getByRole('button', { name: /Build from scratch/i });
    await user.click(scratchBtn);

    expect(onNext).toHaveBeenCalledTimes(1);
    expect(applyPreset).not.toHaveBeenCalled();
  });
});

// ------------------------------------------------------------------
// Test 5: HERO_PORTRAIT map exposes the 4 hero classes
// ------------------------------------------------------------------
describe('HERO_PORTRAIT', () => {
  it('exposes portrait entries for all 4 hero classes', () => {
    const classes = ['fighter', 'wizard', 'rogue', 'cleric'] as const;
    for (const cls of classes) {
      expect(HERO_PORTRAIT[cls]).toBeDefined();
      expect(typeof HERO_PORTRAIT[cls]).toBe('string');
    }
  });

  it('paladin entry is also present (CharacterSheet compat)', () => {
    expect(HERO_PORTRAIT.paladin).toBeDefined();
  });
});
