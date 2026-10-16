import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import '../../i18n';
import { useStore } from '../../state/useStore';
import { SceneTransitionOverlay } from '../SceneTransitionOverlay';

const initial = { scopeKey: 'campaign:timeline-1:archive', active: false, encounterId: null };
let reduced = false;
let changed: (() => void) | undefined;
beforeEach(() => {
  vi.useFakeTimers();
  reduced = false;
  changed = undefined;
  vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  vi.stubGlobal('matchMedia', () => ({
    get matches() {
      return reduced;
    },
    addEventListener: (_event: string, listener: () => void) => {
      changed = listener;
    },
    removeEventListener: vi.fn(),
  }));
  useStore.setState(useStore.getInitialState());
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it('does not display a dialog or take composer focus on ordinary movement', () => {
  const view = render(
    <>
      <input aria-label="Composer" />
      <SceneTransitionOverlay {...initial} />
    </>,
  );
  screen.getByLabelText('Composer').focus();
  const timersBeforeMove = vi.getTimerCount();
  view.rerender(
    <>
      <input aria-label="Composer" />
      <SceneTransitionOverlay {...initial} scopeKey="campaign:timeline-1:quay" />
    </>,
  );
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.getByLabelText('Composer')).toHaveFocus();
  expect(vi.getTimerCount()).toBe(timersBeforeMove);
});

it('shows a canonical major moment with selected art and expires without an exit wait', () => {
  const view = render(<SceneTransitionOverlay {...initial} />);
  view.rerender(<SceneTransitionOverlay {...initial} active encounterId="fight-1" />);
  expect(screen.getByTestId('scene-transition-art')).toHaveAttribute(
    'data-asset-id',
    'illustration-ruins',
  );
  expect(screen.queryByRole('dialog')).toBeNull();
  act(() => vi.advanceTimersByTime(3200));
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  view.rerender(<SceneTransitionOverlay {...initial} active encounterId="fight-1" />);
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
});

it('keeps the supplied public location binding and dismisses immediately', () => {
  const view = render(<SceneTransitionOverlay {...initial} />);
  view.rerender(
    <SceneTransitionOverlay
      {...initial}
      active
      encounterId="fight-1"
      art={{ id: 'scene.bell.archive.v2', uri: '/public-archive.webp' }}
    />,
  );
  expect(screen.getByTestId('scene-transition-art')).toHaveAttribute(
    'data-asset-id',
    'scene.bell.archive.v2',
  );
  fireEvent.click(screen.getByRole('button', { name: /skip|пропустить/i }));
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  expect(vi.getTimerCount()).toBe(0);
});

it('suppresses initial hydration, restored timeline and reduced/background replay', () => {
  const hydrated = { ...initial, active: true, encounterId: 'fight-1' };
  const view = render(<SceneTransitionOverlay {...hydrated} />);
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  view.rerender(<SceneTransitionOverlay {...hydrated} scopeKey="restored:timeline-2" />);
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  view.unmount();
  const next = render(<SceneTransitionOverlay {...initial} />);
  act(() => {
    reduced = true;
    changed?.();
  });
  next.rerender(<SceneTransitionOverlay {...initial} active encounterId="fight-2" />);
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  act(() => {
    reduced = false;
    changed?.();
  });
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
});

it('cleans the current moment on blur and does not replay on focus or preference toggle', () => {
  const focused = vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  const view = render(<SceneTransitionOverlay {...initial} />);
  view.rerender(<SceneTransitionOverlay {...initial} active encounterId="fight-1" />);
  expect(screen.getByTestId('scene-transition-art')).toBeInTheDocument();
  act(() => {
    focused.mockReturnValue(false);
    window.dispatchEvent(new Event('blur'));
  });
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  expect(vi.getTimerCount()).toBe(0);
  act(() => {
    focused.mockReturnValue(true);
    window.dispatchEvent(new Event('focus'));
  });
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  act(() => useStore.getState().settings.setSceneTransitionsEnabled(false));
  view.rerender(<SceneTransitionOverlay {...initial} encounterId="fight-1" />);
  expect(screen.queryByTestId('scene-transition-art')).toBeNull();
  view.unmount();
  expect(vi.getTimerCount()).toBe(0);
});
