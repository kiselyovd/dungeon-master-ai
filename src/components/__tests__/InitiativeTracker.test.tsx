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
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} visible={true} />);
    expect(screen.getByText('Hero')).toBeTruthy();
    expect(screen.getByText('Goblin')).toBeTruthy();
  });

  it('shows round number', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={3} visible={true} />);
    expect(screen.getByText(/Round 3|Раунд 3/)).toBeTruthy();
  });

  it('marks active combatant', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} visible={true} />);
    const active = document.querySelector('[data-active="true"]');
    expect(active).toBeTruthy();
  });

  it('slides in when visible=true', () => {
    const { container } = render(
      <InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} visible={true} />,
    );
    expect(container.querySelector('.initiative-tracker.visible')).toBeTruthy();
  });

  it('renders HP bar with current/max', () => {
    render(<InitiativeTracker tokens={tokens} order={['a', 'b']} round={1} visible={true} />);
    expect(screen.getByText('15/15')).toBeTruthy();
    expect(screen.getByText('7/7')).toBeTruthy();
  });
});
