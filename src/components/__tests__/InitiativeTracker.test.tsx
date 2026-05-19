import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CombatToken } from '../../state/combat';
import '../../i18n';
import { InitiativeTracker } from '../InitiativeTracker';

const tokens: CombatToken[] = [
  { id: 'a', name: 'Hero', hp: 15, maxHp: 15, ac: 14, x: 0, y: 0, conditions: [], isActive: true },
  { id: 'b', name: 'Goblin', hp: 7, maxHp: 7, ac: 13, x: 3, y: 2, conditions: [] },
];

describe('InitiativeTracker', () => {
  it('renders all combatant names', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} activeTokenId="a" />);
    expect(screen.getByText('Hero')).toBeTruthy();
    expect(screen.getByText('Goblin')).toBeTruthy();
  });

  it('shows round number', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={3} activeTokenId="a" />);
    expect(screen.getByText(/Round 3|Раунд 3/)).toBeTruthy();
  });

  it('marks the active combatant', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} activeTokenId="a" />);
    const active = document.querySelector('[data-active="true"]');
    expect(active).toBeTruthy();
  });

  it('returns null when initiative order is empty', () => {
    const { container } = render(
      <InitiativeTracker tokens={tokens} order={[]} round={1} activeTokenId={null} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders HP digits for each combatant', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} activeTokenId="a" />);
    expect(screen.getByText('15/15')).toBeTruthy();
    expect(screen.getByText('7/7')).toBeTruthy();
  });

  it('clicking a row calls onSelect with the correct token id', () => {
    const onSelect = vi.fn();
    render(
      <InitiativeTracker
        tokens={tokens}
        order={['a', 'b']}
        round={1}
        activeTokenId="a"
        onSelect={onSelect}
      />,
    );
    const goblinButton = screen.getByText('Goblin').closest('button');
    expect(goblinButton).toBeTruthy();
    if (goblinButton) fireEvent.click(goblinButton);
    expect(onSelect).toHaveBeenCalledWith('b');
  });

  it('active card has is-active class and inactive card does not', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} activeTokenId="a" />);
    const cards = document.querySelectorAll('.dm-init-card');
    expect(cards[0]?.classList.contains('is-active')).toBe(true);
    expect(cards[1]?.classList.contains('is-active')).toBe(false);
  });
});
