import { listen } from '@tauri-apps/api/event';
import { expect, it, vi } from 'vitest';
import { getCachedBackendPort, initBackendListener, subscribeBackendReady } from '../client';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));
it('backend readiness updates the transport before notifying configuration restoration and supports cleanup', async () => {
  let handler!: (event: { payload: number }) => void;
  vi.mocked(listen).mockImplementation(async (_name, callback) => {
    handler = callback as typeof handler;
    return () => {};
  });
  const observed: number[] = [];
  const detach = subscribeBackendReady(() => {
    const port = getCachedBackendPort();
    if (port !== null) observed.push(port);
  });
  const stop = await initBackendListener();
  handler({ payload: 7777 });
  expect(observed).toEqual([7777]);
  detach();
  handler({ payload: 8888 });
  expect(observed).toEqual([7777]);
  expect(getCachedBackendPort()).toBe(8888);
  stop();
});
