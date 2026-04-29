import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { ActionBar } from '../ActionBar';

describe('ActionBar', () => {
  it('renders all four action buttons + end-turn', () => {
    render(
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        visible={true}
        onEndTurn={() => {}}
      />,
    );
    expect(screen.getByTestId('action-btn-action')).toBeTruthy();
    expect(screen.getByTestId('action-btn-bonus')).toBeTruthy();
    expect(screen.getByTestId('action-btn-reaction')).toBeTruthy();
    expect(screen.getByTestId('action-btn-move')).toBeTruthy();
    expect(screen.getByTestId('action-btn-end-turn')).toBeTruthy();
  });

  it('action button is disabled when actionUsed=true', () => {
    render(
      <ActionBar
        actionUsed={true}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        visible={true}
        onEndTurn={() => {}}
      />,
    );
    const btn = screen.getByTestId('action-btn-action') as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
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
        visible={true}
        onEndTurn={onEnd}
      />,
    );
    fireEvent.click(screen.getByTestId('action-btn-end-turn'));
    expect(onEnd).toHaveBeenCalledOnce();
  });

  it('slides up when visible=true', () => {
    const { container } = render(
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        visible={true}
        onEndTurn={() => {}}
      />,
    );
    expect(container.querySelector('.action-bar.visible')).toBeTruthy();
  });
});
