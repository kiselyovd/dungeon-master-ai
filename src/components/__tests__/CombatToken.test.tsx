import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { CombatToken as TokenData } from '../../state/combat';
import { CombatToken } from '../CombatToken';

const baseToken: TokenData = {
  id: 'tok-1',
  name: 'Hero',
  hp: 10,
  maxHp: 15,
  ac: 14,
  x: 2,
  y: 3,
  conditions: [],
  isActive: false,
};

describe('CombatToken', () => {
  it('renders without crashing and exposes the token id', () => {
    const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} />);
    expect(getByTestId('combat-token-tok-1')).toBeTruthy();
  });

  it('renders AC chip with the AC value', () => {
    const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} />);
    const ac = getByTestId('combat-token-tok-1-ac');
    expect(ac.textContent).toBe('14');
  });

  it('active token has data-active attribute', () => {
    const token = { ...baseToken, isActive: true };
    const { getByTestId } = render(<CombatToken token={token} cellSize={30} />);
    const el = getByTestId('combat-token-tok-1');
    expect(el.dataset.active).toBe('true');
  });

  it('hp bar width reflects current/max ratio', () => {
    const token = { ...baseToken, hp: 5, maxHp: 15 };
    const { getByTestId } = render(<CombatToken token={token} cellSize={30} />);
    const bar = getByTestId('combat-token-tok-1-hpbar') as HTMLElement;
    expect(bar.style.width).toBe(`${(5 / 15) * 100}%`);
  });

  it('renders condition dots up to 3 visible', () => {
    const token = {
      ...baseToken,
      conditions: ['prone', 'poisoned', 'frightened', 'grappled'],
    };
    const { container } = render(<CombatToken token={token} cellSize={30} />);
    const dots = container.querySelectorAll('span[title]');
    expect(dots.length).toBe(3);
  });
});
