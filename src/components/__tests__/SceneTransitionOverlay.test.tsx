import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SceneTransitionOverlay } from '../SceneTransitionOverlay';

function changeScene(name: string): void {
  act(() => {
    useStore.setState((state) => ({
      session: { ...state.session, currentScene: { name, stepCounter: 1 } },
    }));
  });
}

function renderAfterScene(name = 'Quiet village'): ReturnType<typeof render> {
  useStore.setState((state) => ({
    session: { ...state.session, currentScene: { name, stepCounter: 1 } },
  }));
  return render(<SceneTransitionOverlay />);
}

describe('SceneTransitionOverlay', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-08-12T10:00:00Z'));
    useStore.setState(useStore.getInitialState());
    useStore.setState((state) => ({
      settings: { ...state.settings, sceneTransitionsEnabled: true },
      session: { ...state.session, currentScene: null },
    }));
  });

  afterEach(() => {
    cleanup();
    vi.clearAllTimers();
    vi.useRealTimers();
  });

  it('cleans pending timers when unmounted', () => {
    const { unmount } = renderAfterScene();
    changeScene('Combat beyond the walls');
    expect(screen.getByTestId('scene-transition-art')).toBeInTheDocument();
    expect(vi.getTimerCount()).toBeGreaterThan(0);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('shows semantic still art and dismisses after its calm display window', () => {
    renderAfterScene();
    changeScene('Combat at the old bridge');

    expect(screen.getByTestId('scene-transition-art')).toHaveAttribute(
      'src',
      expect.stringContaining('scene-combat'),
    );

    act(() => vi.advanceTimersByTime(3_200));
    expect(screen.getByRole('dialog')).toHaveClass('is-fading');

    act(() => vi.advanceTimersByTime(280));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows the first scene created after an initially empty session', () => {
    render(<SceneTransitionOverlay />);
    changeScene('Combat at the old bridge');
    expect(screen.getByTestId('scene-transition-art')).toHaveAttribute(
      'src',
      expect.stringContaining('scene-combat'),
    );
  });

  it('supports Escape and the skip button', () => {
    const { rerender } = renderAfterScene();
    changeScene('Ancient crypt');
    fireEvent.keyDown(window, { key: 'Escape' });
    act(() => vi.advanceTimersByTime(280));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    act(() => vi.advanceTimersByTime(30_000));
    changeScene('Forest road');
    rerender(<SceneTransitionOverlay />);
    fireEvent.click(screen.getByRole('button', { name: /skip|пропустить/i }));
    act(() => vi.advanceTimersByTime(280));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('respects the debounce and disabled setting', () => {
    renderAfterScene();
    changeScene('Battle outside the gate');
    fireEvent.click(screen.getByRole('button', { name: /skip|пропустить/i }));
    act(() => vi.advanceTimersByTime(280));

    changeScene('Dialog with the council');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    act(() => vi.advanceTimersByTime(30_000));
    act(() => useStore.getState().settings.setSceneTransitionsEnabled(false));
    changeScene('Dungeon below the keep');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});
