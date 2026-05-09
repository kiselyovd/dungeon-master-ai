import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { CharacterSheet } from '../CharacterSheet';

/**
 * CharacterSheet test suite (M5 P2.14).
 *
 * Mounts the modal directly and seeds the PC slice via the canonical
 * `applyPreset` action so the rendered values come from the real preset
 * tables in pc.ts. Each test starts from the initial store state.
 */

describe('CharacterSheet', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('renders nothing when open is false', () => {
    const { container } = render(<CharacterSheet open={false} onClose={() => {}} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders the empty state when no character has been created yet', () => {
    render(<CharacterSheet open onClose={() => {}} />);
    expect(screen.getByText(/No character yet/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Go to onboarding/i })).toBeInTheDocument();
  });

  it('"Go to onboarding" resets the onboarding flag and closes the modal', async () => {
    const user = userEvent.setup();
    // Pretend the user already finished onboarding once.
    useStore.getState().onboarding.complete();
    expect(useStore.getState().onboarding.completed).toBe(true);

    const onClose = vi.fn();
    render(<CharacterSheet open onClose={onClose} />);
    await user.click(screen.getByRole('button', { name: /Go to onboarding/i }));

    expect(useStore.getState().onboarding.completed).toBe(false);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('renders the full sheet for a Fighter preset', () => {
    useStore.getState().pc.applyPreset('fighter');
    render(<CharacterSheet open onClose={() => {}} />);

    // Header
    expect(screen.getByRole('heading', { name: /Hero/i, level: 2 })).toBeInTheDocument();
    expect(screen.getByText(/Human/i)).toBeInTheDocument();
    // Combat block
    expect(screen.getByText('Hit Points')).toBeInTheDocument();
    expect(screen.getByText('12 / 12')).toBeInTheDocument();
    expect(screen.getByText('Armor Class')).toBeInTheDocument();
    // Ability grid renders all six labels
    for (const label of ['STR', 'DEX', 'CON', 'INT', 'WIS', 'CHA']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
    // STR 16 -> +3 mod (fighter preset)
    expect(screen.getByText('+3')).toBeInTheDocument();
    // Inventory items appear
    expect(screen.getByText('Longsword')).toBeInTheDocument();
    expect(screen.getByText('Shield')).toBeInTheDocument();
  });

  it('renders skill proficiency markers for the Fighter preset', () => {
    useStore.getState().pc.applyPreset('fighter');
    const { container } = render(<CharacterSheet open onClose={() => {}} />);
    // Two proficient skills out of twelve.
    const profSkills = container.querySelectorAll('.dm-skill.is-prof');
    expect(profSkills.length).toBe(2);
  });

  it('Escape closes the modal', async () => {
    useStore.getState().pc.applyPreset('fighter');
    const onClose = vi.fn();
    render(<CharacterSheet open onClose={onClose} />);
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('clicking the backdrop closes the modal', async () => {
    useStore.getState().pc.applyPreset('wizard');
    const onClose = vi.fn();
    render(<CharacterSheet open onClose={onClose} />);
    const backdrop = screen.getByRole('dialog');
    await userEvent.click(backdrop);
    expect(onClose).toHaveBeenCalled();
  });
});
