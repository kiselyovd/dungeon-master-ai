import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { fetchLocalRuntimeStatus } from '../api/localRuntime';
import { useStore } from '../state/useStore';
import { useLocalRuntimeStatus } from './useLocalRuntimeStatus';

vi.mock('../api/localRuntime', () => ({ fetchLocalRuntimeStatus: vi.fn() }));
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.clearAllMocks();
});
it('invalidates an old running claim when the native status boundary is unavailable', async () => {
  useStore
    .getState()
    .localMode.setRuntimeStatus({ llm: { state: 'ready', port: 34567 }, image: { state: 'off' } });
  vi.mocked(fetchLocalRuntimeStatus).mockRejectedValue(new Error('offline'));
  renderHook(() => useLocalRuntimeStatus(true));
  await waitFor(() => expect(useStore.getState().localMode.runtimeStatusKnown).toBe(false));
  expect(useStore.getState().localMode.runtime.llm).toEqual({ state: 'ready', port: 34567 });
});
it('ignores a late status response after its presentation scope closes', async () => {
  let resolve: (value: Awaited<ReturnType<typeof fetchLocalRuntimeStatus>>) => void = () => {};
  vi.mocked(fetchLocalRuntimeStatus).mockReturnValue(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const hook = renderHook(() => useLocalRuntimeStatus(true));
  hook.unmount();
  await act(async () => resolve({ llm: { state: 'ready', port: 34567 }, image: { state: 'off' } }));
  expect(useStore.getState().localMode.runtimeStatusKnown).toBe(false);
});
