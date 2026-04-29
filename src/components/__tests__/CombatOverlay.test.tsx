import { render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { CombatToken } from '../../state/combat';
import { CombatOverlay } from '../CombatOverlay';

const sampleTokens: CombatToken[] = [
  { id: 'a', name: 'Hero', hp: 15, maxHp: 15, ac: 14, x: 0, y: 0, conditions: [] },
];

describe('CombatOverlay', () => {
  it('renders when active=true with the active class', () => {
    const { getByTestId } = render(
      <CombatOverlay
        active={true}
        tokens={sampleTokens}
        cellSize={30}
        widthCells={20}
        heightCells={20}
      />,
    );
    const el = getByTestId('combat-overlay');
    expect(el.className).toContain('active');
    expect(el.dataset['active']).toBe('true');
  });

  it('omits the active class when active=false', () => {
    const { getByTestId } = render(
      <CombatOverlay active={false} tokens={[]} cellSize={30} widthCells={20} heightCells={20} />,
    );
    const el = getByTestId('combat-overlay');
    expect(el.dataset['active']).toBeUndefined();
    expect(el.className).not.toContain(' active');
  });

  it('renders one CombatToken per token entry', () => {
    const tokens: CombatToken[] = [
      { id: 'a', name: 'Hero', hp: 15, maxHp: 15, ac: 14, x: 0, y: 0, conditions: [] },
      { id: 'b', name: 'Goblin', hp: 7, maxHp: 7, ac: 13, x: 3, y: 2, conditions: [] },
    ];
    const { getByTestId } = render(
      <CombatOverlay
        active={true}
        tokens={tokens}
        cellSize={30}
        widthCells={20}
        heightCells={20}
      />,
    );
    expect(getByTestId('combat-token-a')).toBeTruthy();
    expect(getByTestId('combat-token-b')).toBeTruthy();
  });
});
