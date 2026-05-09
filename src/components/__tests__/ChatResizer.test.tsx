import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { MAX_CHAT_WIDTH, MIN_CHAT_WIDTH } from '../../state/settings';
import { useStore } from '../../state/useStore';
import { ChatResizer } from '../ChatResizer';
import '../../i18n';

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
});
