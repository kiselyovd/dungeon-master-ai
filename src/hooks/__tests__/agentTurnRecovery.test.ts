import { act, renderHook } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { streamAgentTurn } from '../../api/agent';
import { ChatError } from '../../api/errors';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

vi.mock('../../api/agent', () => ({ streamAgentTurn: vi.fn() }));
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.resetAllMocks();
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
