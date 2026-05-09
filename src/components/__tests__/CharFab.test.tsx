import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { CharFab } from '../CharFab';

/**
 * CharFab test suite (M5 P2.14).
 *
 * The fab pill mounts only when a character has been created, then opens
 * the CharacterSheet modal via the parent-supplied callback.
 */

describe('CharFab', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
  });

  it('renders nothing when no character exists yet', () => {
    const { container } = render(<CharFab onOpen={() => {}} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('renders the pill with name and HP once a preset is applied', () => {
    useStore.getState().pc.applyPreset('fighter');
    render(<CharFab onOpen={() => {}} />);
    const btn = screen.getByRole('button', { name: /Open character sheet for Hero/i });
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveTextContent('Hero');
    expect(btn).toHaveTextContent('12/12');
  });

  it('clicking the fab calls onOpen', async () => {
    useStore.getState().pc.applyPreset('rogue');
    const onOpen = vi.fn();
    render(<CharFab onOpen={onOpen} />);
    await userEvent.click(screen.getByRole('button', { name: /Open character sheet/i }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });
});
