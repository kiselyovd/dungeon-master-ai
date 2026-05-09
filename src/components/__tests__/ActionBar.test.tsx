import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { ActionBar } from '../ActionBar';

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

  it('end-turn is disabled when no handler is wired', () => {
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
    expect(btn.disabled).toBe(true);
  });
});
