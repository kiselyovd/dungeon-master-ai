import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { ActionBar } from '../ActionBar';

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
});

describe('ActionBar', () => {
  it('renders the design-spec eight action buttons + economy chips', () => {
    render(
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        onEndTurn={() => {}}
      />,
    );
    expect(screen.getByTestId('action-btn-attack')).toBeTruthy();
    expect(screen.getByTestId('action-btn-cast')).toBeTruthy();
    expect(screen.getByTestId('action-btn-move')).toBeTruthy();
    expect(screen.getByTestId('action-btn-dash')).toBeTruthy();
    expect(screen.getByTestId('action-btn-dodge')).toBeTruthy();
    expect(screen.getByTestId('action-btn-disengage')).toBeTruthy();
    expect(screen.getByTestId('action-btn-use_object')).toBeTruthy();
    expect(screen.getByTestId('action-btn-end_turn')).toBeTruthy();
  });

  it('attack and other action-economy buttons are disabled when actionUsed=true', () => {
    render(
      <ActionBar
        actionUsed={true}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        onEndTurn={() => {}}
      />,
    );
    const attack = screen.getByTestId('action-btn-attack') as HTMLButtonElement;
    expect(attack.disabled).toBe(true);
  });

  it('calls onEndTurn when end-turn is clicked', () => {
    const onEnd = vi.fn();
    render(
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        onEndTurn={onEnd}
      />,
    );
    fireEvent.click(screen.getByTestId('action-btn-end_turn'));
    expect(onEnd).toHaveBeenCalledOnce();
  });

  it('end-turn defaults to store endTurn when no prop is passed (enabled)', () => {
    render(
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
      />,
    );
    const btn = screen.getByTestId('action-btn-end_turn') as HTMLButtonElement;
    // Post-A3: store.combat.endTurn is always defined, so end-turn is enabled
    // even when the onEndTurn prop is omitted. The disabled-fallback path is
    // gone by design (see task A3 spec).
    expect(btn.disabled).toBe(false);
  });

  it('reads actionUsed from CombatSlice when rendered without props', () => {
    useStore.setState((s) => ({ ...s, combat: { ...s.combat, actionUsed: true } }));
    render(<ActionBar />);
    const attack = screen.getByTestId('action-btn-attack') as HTMLButtonElement;
    expect(attack.disabled).toBe(true);
  });

  it('move button title shows remaining/total ft from store when no prop passed', () => {
    useStore.setState((s) => ({
      ...s,
      combat: { ...s.combat, movementRemaining: 15 },
      pc: { ...s.pc, speedFt: 30 },
    }));
    render(<ActionBar />);
    const move = screen.getByTestId('action-btn-move') as HTMLButtonElement;
    expect(move.title).toMatch(/15/);
    expect(move.title).toMatch(/30/);
  });

  it('end-turn button calls combat.endTurn from store when clicked without prop', () => {
    const endTurnSpy = vi.fn();
    useStore.setState((s) => ({
      ...s,
      combat: { ...s.combat, active: true, endTurn: endTurnSpy },
    }));
    render(<ActionBar />);
    fireEvent.click(screen.getByTestId('action-btn-end_turn'));
    expect(endTurnSpy).toHaveBeenCalledOnce();
  });
});
