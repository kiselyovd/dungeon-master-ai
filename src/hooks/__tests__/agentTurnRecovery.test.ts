import { act, renderHook } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { streamAgentTurn } from '../../api/agent';
import { ChatError } from '../../api/errors';
import { prepareBackendProvider } from '../../app/providerStartup';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

vi.mock('../../app/providerStartup', () => ({ prepareBackendProvider: vi.fn(async () => {}) }));
vi.mock('../../api/agent', () => ({ streamAgentTurn: vi.fn() }));
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.resetAllMocks();
  vi.mocked(prepareBackendProvider).mockResolvedValue();
  useStore.getState().session.activate('campaign', 'session');
});
it('retries the immutable turn identity and removes its incomplete local narration', async () => {
  vi.mocked(streamAgentTurn)
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'text_delta', text: 'Incomplete reply' });
      throw new ChatError('network', 'lost');
    })
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'text_delta', text: 'Confirmed reply' });
      opts.onEvent({ type: 'agent_done', totalRounds: 1 });
    });
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('I inspect the quay');
  });
  const first = vi.mocked(streamAgentTurn).mock.calls[0]?.[0];
  await act(async () => {
    await result.current.send('I inspect the quay');
  });
  const second = vi.mocked(streamAgentTurn).mock.calls[1]?.[0];
  expect(first?.requestId).toBeTruthy();
  expect(second?.requestId).toBe(first?.requestId);
  expect(second?.history).toEqual(first?.history);
  expect(useStore.getState().chat.messages.map((m) => m.content)).toEqual([
    'I inspect the quay',
    'Confirmed reply',
  ]);
});

it('configuration failure makes no DM request and its retry starts instead of resuming an unissued turn', async () => {
  vi.mocked(prepareBackendProvider).mockRejectedValueOnce(
    new ChatError('provider_error', 'configuration_failed'),
  );
  vi.mocked(streamAgentTurn).mockResolvedValue();
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('Look at the river');
  });
  expect(streamAgentTurn).not.toHaveBeenCalled();
  await act(async () => {
    await result.current.send('Look at the river');
  });
  expect(streamAgentTurn).toHaveBeenCalledTimes(1);
  expect(vi.mocked(streamAgentTurn).mock.calls[0]?.[0].resume).toBeUndefined();
});

it('cancel releases a turn waiting for provider setup without issuing a DM request', async () => {
  let release!: () => void;
  vi.mocked(prepareBackendProvider).mockImplementationOnce(
    () =>
      new Promise<void>((r) => {
        release = r;
      }),
  );
  const { result } = renderHook(() => useAgentTurn());
  let sending!: Promise<void>;
  act(() => {
    sending = result.current.send('Look at the river');
  });
  expect(useStore.getState().chat.isStreaming).toBe(true);
  await act(async () => {
    result.current.cancel();
    await sending;
  });
  expect(useStore.getState().chat.isStreaming).toBe(false);
  expect(streamAgentTurn).not.toHaveBeenCalled();
  release();
  await Promise.resolve();
  expect(streamAgentTurn).not.toHaveBeenCalled();
});
