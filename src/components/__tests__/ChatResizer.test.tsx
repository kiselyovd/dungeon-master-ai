import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { MAX_CHAT_WIDTH, MIN_CHAT_WIDTH } from '../../state/settings';
import { useStore } from '../../state/useStore';
import { ChatResizer } from '../ChatResizer';
import '../../i18n';

const RESIZING_BODY_CLASS = 'dm-chat-resizing';

// jsdom does not implement setPointerCapture / releasePointerCapture; stub
// them so the component's pointerdown handler does not throw.
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
  useStore.setState(useStore.getInitialState());
});

describe('ChatResizer', () => {
  it('renders a vertical separator with an aria label and the current width', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');
    expect(handle).toHaveAttribute('aria-orientation', 'vertical');
    expect(handle).toHaveAttribute('aria-valuemin', String(MIN_CHAT_WIDTH));
    expect(handle).toHaveAttribute('aria-valuemax', String(MAX_CHAT_WIDTH));
    expect(handle).toHaveAttribute('aria-valuenow', '480');
    expect(handle.getAttribute('aria-label')).toBeTruthy();
  });

  it('updates persisted width within range when a pointer drag completes', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    // Start drag at clientX=1000 with current width=480.
    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    // Drag 60px to the LEFT -> chat should grow by 60px to 540.
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 940 });
    fireEvent.pointerUp(handle, { pointerId: 1, clientX: 940 });

    expect(useStore.getState().settings.chatPanelWidth).toBe(540);
  });

  it('clamps to MAX when the drag would push past the upper bound', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    // Drag 500px LEFT -> 480 + 500 = 980, must clamp to MAX_CHAT_WIDTH.
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 500 });
    fireEvent.pointerUp(handle, { pointerId: 1, clientX: 500 });

    expect(useStore.getState().settings.chatPanelWidth).toBe(MAX_CHAT_WIDTH);
  });

  it('clamps to MIN when the drag would push past the lower bound', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 500 });
    // Drag 500px RIGHT -> 480 - 500 = -20, must clamp to MIN_CHAT_WIDTH.
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 1000 });
    fireEvent.pointerUp(handle, { pointerId: 1, clientX: 1000 });

    expect(useStore.getState().settings.chatPanelWidth).toBe(MIN_CHAT_WIDTH);
  });

  it('ArrowLeft widens the chat by 16px and ArrowRight narrows it', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.keyDown(handle, { key: 'ArrowLeft' });
    expect(useStore.getState().settings.chatPanelWidth).toBe(496);

    fireEvent.keyDown(handle, { key: 'ArrowRight' });
    fireEvent.keyDown(handle, { key: 'ArrowRight' });
    expect(useStore.getState().settings.chatPanelWidth).toBe(464);
  });

  it('Home/End jump to MIN/MAX bounds', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.keyDown(handle, { key: 'End' });
    expect(useStore.getState().settings.chatPanelWidth).toBe(MAX_CHAT_WIDTH);

    fireEvent.keyDown(handle, { key: 'Home' });
    expect(useStore.getState().settings.chatPanelWidth).toBe(MIN_CHAT_WIDTH);
  });

  it('Shift+ArrowLeft adjusts width by the large 64px step', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.keyDown(handle, { key: 'ArrowLeft', shiftKey: true });
    // Default 480 + 64 = 544.
    expect(useStore.getState().settings.chatPanelWidth).toBe(544);

    fireEvent.keyDown(handle, { key: 'ArrowRight', shiftKey: true });
    // 544 - 64 = 480.
    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
  });

  it('pointercancel cleans up body class and commits the cancelled width', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 940 });
    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(true);

    fireEvent.pointerCancel(handle, { pointerId: 1, clientX: 940 });

    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
    // pointercancel reverts to startWidth (480), it does not commit the
    // intermediate drag position.
    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
  });

  it('Escape during a drag reverts to the start width and clears body class', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 900 });
    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(true);

    fireEvent.keyDown(handle, { key: 'Escape' });

    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
  });

  it('Escape outside an active drag is a no-op', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.keyDown(handle, { key: 'Escape' });

    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
  });

  it('window blur during a drag triggers cancel cleanup', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 900 });
    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(true);

    // jsdom dispatches the global blur event synchronously.
    act(() => {
      window.dispatchEvent(new Event('blur'));
    });

    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
    // Cancelled drag reverts to start width.
    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
  });

  it('document visibilitychange during a drag triggers cancel cleanup', () => {
    render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    fireEvent.pointerMove(handle, { pointerId: 1, clientX: 900 });

    act(() => {
      document.dispatchEvent(new Event('visibilitychange'));
    });

    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
    expect(useStore.getState().settings.chatPanelWidth).toBe(480);
  });

  it('unmount during an active drag removes the body class', () => {
    const { unmount } = render(<ChatResizer />);
    const handle = screen.getByRole('separator');

    fireEvent.pointerDown(handle, { button: 0, pointerId: 1, clientX: 1000 });
    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(true);

    unmount();

    expect(document.body.classList.contains(RESIZING_BODY_CLASS)).toBe(false);
  });
});
