import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { ActionBar } from '../ActionBar';

beforeEach(() => useStore.setState(useStore.getInitialState()));

describe('ActionBar authoritative projection behavior', () => {
  it('renders the eight established controls and economy chips', () => {
    render(<ActionBar movementFt={30} speedFt={30} onEndTurn={() => {}} />);
    for (const action of [
      'attack',
      'cast',
      'move',
      'dash',
      'dodge',
      'disengage',
      'use_object',
      'end_turn',
    ]) {
      expect(screen.getByTestId(`action-btn-${action}`)).toBeTruthy();
    }
  });

  it('derives disabled action state only from the authoritative budget', () => {
    render(<ActionBar actionUsed movementFt={30} speedFt={30} onEndTurn={() => {}} />);
    expect(screen.getByTestId('action-btn-attack')).toBeDisabled();
    expect(screen.getByTestId('action-btn-cast')).toBeDisabled();
  });

  it('does not consume action locally before emitting an intent', () => {
    const onIntent = vi.fn();
    render(
      <ActionBar
        actionUsed={false}
        movementFt={30}
        speedFt={30}
        onIntent={onIntent}
        onEndTurn={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId('action-btn-attack'));
    expect(onIntent).toHaveBeenCalledOnce();
    expect(useStore.getState().combat.actionUsed).toBe(false);
  });

  it('disables all command controls while an authoritative request is pending', () => {
    render(<ActionBar pending movementFt={30} speedFt={30} onEndTurn={() => {}} />);
    expect(screen.getByTestId('action-btn-attack')).toBeDisabled();
    expect(screen.getByTestId('action-btn-move')).toBeDisabled();
    expect(screen.getByTestId('action-btn-end_turn')).toBeDisabled();
  });

  it('requires an explicit end-turn command handler', () => {
    const onEndTurn = vi.fn();
    const { rerender } = render(<ActionBar movementFt={30} speedFt={30} />);
    expect(screen.getByTestId('action-btn-end_turn')).toBeDisabled();
    rerender(<ActionBar movementFt={30} speedFt={30} onEndTurn={onEndTurn} />);
    fireEvent.click(screen.getByTestId('action-btn-end_turn'));
    expect(onEndTurn).toHaveBeenCalledOnce();
  });
});
