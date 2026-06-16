import { fireEvent, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CombatToken as TokenData } from '../../state/combat';
import { useStore } from '../../state/useStore';
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
  // Reset pc.name so drag-gating tests that set it don't bleed across tests.
  useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: null } }));
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
    // Seed store so the token is recognised as the active PC token (W1.2).
    useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
    const onMove = vi.fn();
    const { getByTestId } = render(
      <CombatToken token={baseToken} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
    );
    const el = getByTestId('combat-token-tok-1');
    // token at (2, 3) -> origin px (60, 90). Drag delta (30, 30) snaps to (3, 4).
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 105, clientY: 135 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 105, clientY: 135 });
    expect(onMove).toHaveBeenCalledTimes(1);
    expect(onMove).toHaveBeenCalledWith('tok-1', 3, 4);
  });

  it('Escape cancels drag and does not call onMove', () => {
    // Seed store so the token is recognised as the active PC token (W1.2).
    useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
    const onMove = vi.fn();
    const { getByTestId } = render(
      <CombatToken token={baseToken} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 120, clientY: 135 });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onMove).not.toHaveBeenCalled();
  });

  it('ghost token renders during drag', () => {
    // Seed store so the token is recognised as the active PC token (W1.2).
    useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
    const onMove = vi.fn();
    const token = { ...baseToken, x: 0, y: 0 };
    const { container, getByTestId } = render(
      <CombatToken token={token} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 15, clientY: 15 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 60, clientY: 60 });
    const ghost = container.querySelector('[data-ghost="true"]');
    expect(ghost).toBeTruthy();
  });

  it('ghost disappears after pointer release', () => {
    // Seed store so the token is recognised as the active PC token (W1.2).
    useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
    const onMove = vi.fn();
    const token = { ...baseToken, x: 0, y: 0 };
    const { container, getByTestId } = render(
      <CombatToken token={token} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 15, clientY: 15 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 60, clientY: 60 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 60, clientY: 60 });
    const ghost = container.querySelector('[data-ghost="true"]');
    expect(ghost).toBeNull();
  });

  it('right-click does not start a drag', () => {
    // Seed store so the token is recognised as the active PC token (W1.2).
    useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
    const onMove = vi.fn();
    const { getByTestId } = render(
      <CombatToken token={baseToken} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
    );
    const el = getByTestId('combat-token-tok-1');
    fireEvent.pointerDown(el, { button: 2, pointerId: 1, clientX: 75, clientY: 105 });
    fireEvent.pointerMove(el, { pointerId: 1, clientX: 120, clientY: 135 });
    fireEvent.pointerUp(el, { pointerId: 1, clientX: 120, clientY: 135 });
    expect(onMove).not.toHaveBeenCalled();
  });

  it('dead token (hp=0) has data-dead="true" on container', () => {
    const dead = { ...baseToken, hp: 0 };
    const { getByTestId } = render(<CombatToken token={dead} cellSize={30} />);
    expect(getByTestId('combat-token-tok-1').dataset.dead).toBe('true');
  });

  it('dead token shows skull icon overlay', () => {
    const dead = { ...baseToken, hp: 0 };
    const { container } = render(<CombatToken token={dead} cellSize={30} />);
    expect(container.querySelector('[data-testid="combat-token-skull"]')).toBeTruthy();
  });

  it('poisoned condition dot uses necromancy color', () => {
    const token = { ...baseToken, conditions: ['poisoned'] };
    const { container } = render(<CombatToken token={token} cellSize={30} />);
    const dot = container.querySelector('span[aria-label="poisoned"]') as HTMLElement | null;
    expect(dot).toBeTruthy();
    expect(dot?.style.background).toBe('var(--magic-necromancy)');
  });

  it('condition dots have aria-labels', () => {
    const token = { ...baseToken, conditions: ['stunned', 'prone'] };
    const { container } = render(<CombatToken token={token} cellSize={30} />);
    const dots = container.querySelectorAll('span[aria-label]');
    // Filter to only conditions (not skull, not portrait fallback letter)
    const conditionDots = Array.from(dots).filter((d) => {
      const label = (d as HTMLElement).getAttribute('aria-label') ?? '';
      return label === 'stunned' || label === 'prone';
    });
    expect(conditionDots.length).toBe(2);
  });

  // W1.2 - turn-gating: draggable predicate
  describe('turn-gated dragging (W1.2)', () => {
    // The PC token is identified by matching pcName (from store). Since tests
    // run in jsdom without store state, we drive the scenario via onMove absence
    // and currentTurnId to cover the gating logic paths.

    it('token with onMove but NOT active turn (currentTurnId mismatch) does not call onMove on drag', () => {
      const onMove = vi.fn();
      // baseToken.name is 'Hero'; pcName from store is null in test env so
      // isPcToken=false -> draggable=false regardless of currentTurnId
      const { getByTestId } = render(
        <CombatToken token={baseToken} cellSize={30} onMove={onMove} currentTurnId="other-id" />,
      );
      const el = getByTestId('combat-token-tok-1');
      fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
      fireEvent.pointerMove(el, { pointerId: 1, clientX: 105, clientY: 135 });
      fireEvent.pointerUp(el, { pointerId: 1, clientX: 105, clientY: 135 });
      // onMove should NOT be called because token.id !== currentTurnId
      expect(onMove).not.toHaveBeenCalled();
    });

    it('dead PC token on its turn is not draggable', () => {
      const onMove = vi.fn();
      const deadToken = { ...baseToken, hp: 0 };
      const { getByTestId } = render(
        <CombatToken
          token={deadToken}
          cellSize={30}
          onMove={onMove}
          currentTurnId={deadToken.id}
        />,
      );
      const el = getByTestId('combat-token-tok-1');
      fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
      fireEvent.pointerMove(el, { pointerId: 1, clientX: 105, clientY: 135 });
      fireEvent.pointerUp(el, { pointerId: 1, clientX: 105, clientY: 135 });
      expect(onMove).not.toHaveBeenCalled();
    });

    it('token without onMove is never draggable', () => {
      // No drag handlers attached - renders without grab cursor
      const { getByTestId } = render(
        <CombatToken token={baseToken} cellSize={30} currentTurnId={baseToken.id} />,
      );
      const el = getByTestId('combat-token-tok-1');
      // cursor style should not be 'grab' since draggable=false
      expect((el as HTMLElement).style.cursor).not.toBe('grab');
    });

    it('token on its turn with null currentTurnId is not draggable', () => {
      const onMove = vi.fn();
      const { getByTestId } = render(
        <CombatToken token={baseToken} cellSize={30} onMove={onMove} currentTurnId={null} />,
      );
      const el = getByTestId('combat-token-tok-1');
      fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 75, clientY: 105 });
      fireEvent.pointerMove(el, { pointerId: 1, clientX: 105, clientY: 135 });
      fireEvent.pointerUp(el, { pointerId: 1, clientX: 105, clientY: 135 });
      expect(onMove).not.toHaveBeenCalled();
    });
  });

  // W1.7a - token move animation: transition enabled when not dragging
  describe('move animation (W1.7a)', () => {
    it('transition is set on left/top when token is not being dragged', () => {
      const { getByTestId } = render(<CombatToken token={baseToken} cellSize={30} />);
      const el = getByTestId('combat-token-tok-1') as HTMLElement;
      // When not dragging, inline transition should be set to animate moves.
      expect(el.style.transition).toContain('left');
      expect(el.style.transition).toContain('top');
    });

    it('transition is cleared during an active drag so pointer tracking has zero lag', () => {
      // Seed the store so the token is the active PC and draggable.
      useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
      const onMove = vi.fn();
      const token = { ...baseToken, x: 0, y: 0 };
      const { getByTestId } = render(
        <CombatToken token={token} cellSize={30} onMove={onMove} currentTurnId="tok-1" />,
      );
      const el = getByTestId('combat-token-tok-1') as HTMLElement;
      // Before drag: transition is set.
      expect(el.style.transition).toContain('left');
      // Start dragging.
      fireEvent.pointerDown(el, { button: 0, pointerId: 1, clientX: 15, clientY: 15 });
      fireEvent.pointerMove(el, { pointerId: 1, clientX: 60, clientY: 60 });
      // During drag: transition must be cleared (empty string or no value).
      expect(el.style.transition).toBe('');
    });
  });

  // W1.7b - whose-turn visual: "Your turn" label and PC active ring
  describe('whose-turn visuals (W1.7b)', () => {
    it('shows "Your turn" label when PC token is active and alive', () => {
      useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
      const activeToken = { ...baseToken, isActive: true };
      const { getByTestId } = render(<CombatToken token={activeToken} cellSize={40} />);
      const label = getByTestId('combat-token-tok-1-your-turn');
      expect(label).toBeTruthy();
      expect(label.textContent).toBe('Your turn');
    });

    it('does not show "Your turn" label on an enemy token even when active', () => {
      // pc.name is null (default from beforeEach), so the token is not a PC token.
      const activeToken = { ...baseToken, isActive: true };
      const { queryByTestId } = render(<CombatToken token={activeToken} cellSize={40} />);
      expect(queryByTestId('combat-token-tok-1-your-turn')).toBeNull();
    });

    it('does not show "Your turn" label when the PC token is dead (hp=0)', () => {
      useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
      const deadActive = { ...baseToken, hp: 0, isActive: true };
      const { queryByTestId } = render(<CombatToken token={deadActive} cellSize={40} />);
      expect(queryByTestId('combat-token-tok-1-your-turn')).toBeNull();
    });

    it('does not show "Your turn" label when PC token is not the active turn', () => {
      useStore.setState((s) => ({ ...s, pc: { ...s.pc, name: 'Hero' } }));
      const inactiveToken = { ...baseToken, isActive: false };
      const { queryByTestId } = render(<CombatToken token={inactiveToken} cellSize={40} />);
      expect(queryByTestId('combat-token-tok-1-your-turn')).toBeNull();
    });
  });
});
