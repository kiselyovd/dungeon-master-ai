import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchLocalRuntimeStatus } from '../../api/localRuntime';
import { useLocalRuntimeStatus } from '../useLocalRuntimeStatus';

vi.mock('../../api/localRuntime', () => ({ fetchLocalRuntimeStatus: vi.fn() }));

describe('useLocalRuntimeStatus', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('does not overlap status requests when a previous poll is unresolved', async () => {
    let resolveRequest:
      | ((value: { llm: { state: 'off' }; image: { state: 'off' } }) => void)
      | undefined;
    vi.mocked(fetchLocalRuntimeStatus).mockReturnValue(
      new Promise((resolve) => {
        resolveRequest = resolve;
      }),
    );
    renderHook(() => useLocalRuntimeStatus(true));
    expect(fetchLocalRuntimeStatus).toHaveBeenCalledTimes(1);

    await act(async () => vi.advanceTimersByTimeAsync(20_000));
    expect(fetchLocalRuntimeStatus).toHaveBeenCalledTimes(1);

    await act(async () => resolveRequest?.({ llm: { state: 'off' }, image: { state: 'off' } }));
    await act(async () => vi.advanceTimersByTimeAsync(5_000));
    expect(fetchLocalRuntimeStatus).toHaveBeenCalledTimes(2);
  });
});
