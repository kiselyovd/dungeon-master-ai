import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useClosingAnimation } from '../useClosingAnimation';

describe('useClosingAnimation', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('closes immediately and only once in reduced motion without a timer', () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true }));
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose));
    act(() => {
      result.current.triggerClose();
      result.current.triggerClose();
    });
    expect(onClose).toHaveBeenCalledOnce();
    expect(result.current.isClosing).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('finishes a pending close immediately when reduced motion is enabled', () => {
    let reduced = false;
    let listener: (() => void) | undefined;
    vi.stubGlobal('matchMedia', () => ({
      get matches() {
        return reduced;
      },
      addEventListener: (_name: string, callback: () => void) => {
        listener = callback;
      },
      removeEventListener: vi.fn(),
    }));
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose));
    act(() => result.current.triggerClose());
    act(() => {
      reduced = true;
      listener?.();
    });
    expect(onClose).toHaveBeenCalledOnce();
    expect(result.current.isClosing).toBe(false);
    act(() => vi.advanceTimersByTime(280));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('isClosing is false initially', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose));
    expect(result.current.isClosing).toBe(false);
  });

  it('triggerClose sets isClosing to true immediately', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose));
    act(() => {
      result.current.triggerClose();
    });
    expect(result.current.isClosing).toBe(true);
  });

  it('onClose is NOT called before durationMs elapses', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose, 280));
    act(() => {
      result.current.triggerClose();
    });
    act(() => {
      vi.advanceTimersByTime(279);
    });
    expect(onClose).not.toHaveBeenCalled();
  });

  it('onClose IS called once after durationMs elapses', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose, 280));
    act(() => {
      result.current.triggerClose();
    });
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('calling triggerClose twice does not call onClose more than once', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose, 280));
    act(() => {
      result.current.triggerClose();
      result.current.triggerClose();
    });
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('isClosing resets to false after durationMs elapses and onClose fires', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose, 280));
    act(() => {
      result.current.triggerClose();
    });
    expect(result.current.isClosing).toBe(true);
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).toHaveBeenCalledOnce();
    expect(result.current.isClosing).toBe(false);
  });

  it('triggerClose can be called again after a full close cycle', () => {
    const onClose = vi.fn();
    const { result } = renderHook(() => useClosingAnimation(onClose, 280));
    // First close cycle
    act(() => {
      result.current.triggerClose();
    });
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).toHaveBeenCalledTimes(1);
    // Second close cycle - must not be blocked by stale closingRef
    act(() => {
      result.current.triggerClose();
    });
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it('unmounting before the timer fires does NOT call onClose', () => {
    const onClose = vi.fn();
    const { result, unmount } = renderHook(() => useClosingAnimation(onClose, 280));
    act(() => {
      result.current.triggerClose();
    });
    // Unmount before the 280ms timer fires
    unmount();
    act(() => {
      vi.advanceTimersByTime(280);
    });
    expect(onClose).not.toHaveBeenCalled();
  });
});
