import { fireEvent, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
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

beforeEach(() => {
  if (!('setPointerCapture' in HTMLElement.prototype)) {
    (HTMLElement.prototype as unknown as { setPointerCapture: () => void }).setPointerCapture =
      () => {};
  }
  if (!('releasePointerCapture' in HTMLElement.prototype)) {
    (
      HTMLElement.prototype as unknown as { releasePointerCapture: () => void }
    ).releasePointerCapture = () => {};
  }
});

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

  it('calls onMove with snapped grid coordinates on pointer release', () => {
    const onMove = vi.fn();
    const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} onMove={onMove} />);
    const el = getByTestId('combat-token-tok-1');
    // token at (2, 3) -> origin px (60, 90). Drag delta (30, 30) snaps to (3, 4).
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 105, clientY: 135 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 105, clientY: 135 });
    expect(onMove).toHaveBeenCalledTimes(1);
    expect(onMove).toHaveBeenCalledWith('tok-1', 3, 4);
  });

  it('Escape cancels drag and does not call onMove', () => {
    const onMove = vi.fn();
    const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} onMove={onMove} />);
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 120, clientY: 135 });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onMove).not.toHaveBeenCalled();
  });

  it('ghost token renders during drag', () => {
    const onMove = vi.fn();
    const token = { ...baseToken, x: 0, y: 0 };
    const { container, getByTestId } = render(
      <CombatToken token={token} cellSize={30} onMove={onMove} />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 15, clientY: 15 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 60, clientY: 60 });
    const ghost = container.querySelector('[data-ghost="true"]');
    expect(ghost).toBeTruthy();
  });

  it('ghost disappears after pointer release', () => {
    const onMove = vi.fn();
    const token = { ...baseToken, x: 0, y: 0 };
    const { container, getByTestId } = render(
      <CombatToken token={token} cellSize={30} onMove={onMove} />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 15, clientY: 15 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 60, clientY: 60 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 60, clientY: 60 });
    const ghost = container.querySelector('[data-ghost="true"]');
    expect(ghost).toBeNull();
  });

  it('right-click does not start a drag', () => {
    const onMove = vi.fn();
    const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} onMove={onMove} />);
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 2, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 120, clientY: 135 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 120, clientY: 135 });
    expect(onMove).not.toHaveBeenCalled();
  });
});
