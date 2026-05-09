import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
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
});
