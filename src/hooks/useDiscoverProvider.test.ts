import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { postDiscover } from '../api/discovery';
import { useDiscoverProvider } from './useDiscoverProvider';

vi.mock('../api/discovery', () => ({ postDiscover: vi.fn() }));
it('discards late checks for old credentials and enables a new check after editing the draft', async () => {
  let resolve: (value: Awaited<ReturnType<typeof postDiscover>>) => void = () => {};
  vi.mocked(postDiscover).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const hook = renderHook(
    ({ key }) =>
      useDiscoverProvider({
        providerId: 'openai-compat',
        baseUrl: 'http://localhost:1/v1',
        apiKey: key,
      }),
    { initialProps: { key: 'fixture-old' } },
  );
  let pending: Promise<unknown> = Promise.resolve();
  act(() => {
    pending = hook.result.current.discover();
  });
  expect(hook.result.current.status).toBe('loading');
  hook.rerender({ key: 'fixture-new' });
  await waitFor(() => expect(hook.result.current.status).toBe('idle'));
  await act(async () => {
    resolve({ models: [], cached_at: new Date().toISOString(), source: 'discovered-api' });
    await pending;
  });
  expect(hook.result.current.models).toEqual([]);
  expect(hook.result.current.status).toBe('idle');
});
