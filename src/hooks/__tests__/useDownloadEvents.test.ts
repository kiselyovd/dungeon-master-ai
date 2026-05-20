import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as localLlmApi from '../../api/localLlm';
import { useLocalLlmStore } from '../../state/localLlm';
import { useDownloadEvents } from '../useDownloadEvents';

vi.mock('../../api/localLlm', async (importOriginal) => {
  const original = await importOriginal<typeof localLlmApi>();
  return {
    ...original,
    subscribeDownloadEvents: vi.fn(),
  };
});

const subscribeDownloadEventsMock = vi.mocked(localLlmApi.subscribeDownloadEvents);

beforeEach(() => {
  subscribeDownloadEventsMock.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('useDownloadEvents - EventSource leak on fast unmount', () => {
  it('tears down a late-resolving subscription when the component unmounted before the promise resolved', async () => {
    const cancelFn = vi.fn();

    // subscribeDownloadEvents resolves AFTER a microtask, simulating the async
    // backendUrl() call completing after the component has already unmounted.
    subscribeDownloadEventsMock.mockImplementation(
      () =>
        new Promise<() => void>((resolve) => {
          // Defer one microtask so the cleanup runs first.
          void Promise.resolve().then(() => resolve(cancelFn));
        }),
    );

    const { unmount } = renderHook(() => useDownloadEvents());

    // Unmount BEFORE the deferred promise resolves.
    unmount();

    // Flush all pending microtasks - the promise resolves here, and the
    // `if (unmounted)` branch should call cancelFn immediately.
    await act(async () => {
      await Promise.resolve();
    });

    expect(cancelFn).toHaveBeenCalledTimes(1);
  });

  it('tears down the subscription via the cleanup when unmounted after the promise resolves', async () => {
    const cancelFn = vi.fn();

    // subscribeDownloadEvents resolves synchronously in this case (already
    // resolved by the time the `.then` handler runs).
    subscribeDownloadEventsMock.mockResolvedValue(cancelFn);

    const { unmount } = renderHook(() => useDownloadEvents());

    // Let the subscription resolve.
    await act(async () => {
      await Promise.resolve();
    });

    expect(cancelFn).not.toHaveBeenCalled();

    unmount();

    expect(cancelFn).toHaveBeenCalledTimes(1);
  });

  it('logs a warning and does not crash when the subscription rejects', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => undefined);

    subscribeDownloadEventsMock.mockRejectedValue(new Error('network error'));

    const { unmount } = renderHook(() => useDownloadEvents());

    await act(async () => {
      await Promise.resolve();
    });

    expect(warnSpy).toHaveBeenCalledWith(
      '[useDownloadEvents] failed to subscribe:',
      expect.any(Error),
    );

    // Unmounting after a rejected subscription must not throw.
    expect(() => unmount()).not.toThrow();
  });

  it('dispatches incoming events into the localLlm store', async () => {
    let capturedCallback: ((ev: localLlmApi.DownloadEventWire) => void) | undefined;
    const cancelFn = vi.fn();

    subscribeDownloadEventsMock.mockImplementation(async (cb) => {
      capturedCallback = cb;
      return cancelFn;
    });

    renderHook(() => useDownloadEvents());

    await act(async () => {
      await Promise.resolve();
    });

    const ev: localLlmApi.DownloadEventWire = {
      id: 'qwen3.5-4b',
      kind: 'progress',
      bytes_done: 512,
      total_bytes: 1024,
    };

    act(() => {
      capturedCallback?.(ev);
    });

    const ds = useLocalLlmStore.getState().downloadStates.get('qwen3.5-4b');
    expect(ds?.state).toBe('downloading');
    expect(ds?.progress).toBeCloseTo(0.5);
  });
});
