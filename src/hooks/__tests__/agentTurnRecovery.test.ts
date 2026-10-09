import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { streamAgentTurn } from '../../api/agent';
import { getPendingAgentClarification } from '../../api/agentPending';
import { ChatError } from '../../api/errors';
import { prepareBackendProvider } from '../../app/providerStartup';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

vi.mock('../../app/providerStartup', () => ({ prepareBackendProvider: vi.fn(async () => {}) }));
vi.mock('../../api/agent', () => ({ streamAgentTurn: vi.fn() }));
vi.mock('../../api/agentPending', () => ({ getPendingAgentClarification: vi.fn() }));
beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.resetAllMocks();
  vi.mocked(prepareBackendProvider).mockResolvedValue();
  vi.mocked(getPendingAgentClarification).mockResolvedValue(null);
  useStore.getState().session.activate('campaign', 'session');
});
it('recovers a server-owned question after hook remount and keeps the original wire history', async () => {
  const clarificationId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const requestId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
  vi.mocked(getPendingAgentClarification).mockResolvedValueOnce({
    clarificationId,
    input: {
      requestId,
      campaignId: 'campaign',
      sessionId: 'session',
      playerMessage: 'Иду на причал',
      history: [],
      wireHistory: [{ role: 'user', parts: [] }],
    },
  });
  vi.mocked(streamAgentTurn).mockImplementationOnce(async (opts) => {
    opts.onEvent({ type: 'text_delta', text: 'Вы на старом причале.' });
    opts.onEvent({ type: 'agent_done', totalRounds: 1 });
  });
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('На старый');
  });
  const request = vi.mocked(streamAgentTurn).mock.calls[0]?.[0];
  expect(request?.requestId).toBe(requestId);
  expect(request?.playerMessage).toBe('Иду на причал');
  expect(request?.wireHistory).toEqual([{ role: 'user', parts: [] }]);
  expect(request?.clarification).toEqual({ id: clarificationId, reply: 'На старый' });
  expect(request?.resume).toBe(true);
});
it('does not issue a fresh command if protected pending recovery fails', async () => {
  vi.mocked(getPendingAgentClarification).mockRejectedValueOnce(
    new ChatError('network', 'offline'),
  );
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('На старый');
  });
  expect(streamAgentTurn).not.toHaveBeenCalled();
});
it('reprobes after a question text arrives without its terminal event', async () => {
  const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  vi.mocked(streamAgentTurn)
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'text_delta', text: 'Какой причал?' });
      throw new ChatError('network', 'lost_terminal');
    })
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'agent_done', totalRounds: 1 });
    });
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('Иду на причал');
  });
  const original = vi.mocked(streamAgentTurn).mock.calls[0]?.[0];
  if (!original?.requestId) throw new Error('original request required');
  vi.mocked(getPendingAgentClarification).mockResolvedValueOnce({
    clarificationId: id,
    input: {
      requestId: original.requestId,
      campaignId: original.campaignId,
      sessionId: original.sessionId,
      playerMessage: original.playerMessage,
      history: original.history,
    },
  });
  await act(async () => {
    await result.current.send('На старый');
  });
  const resumed = vi.mocked(streamAgentTurn).mock.calls[1]?.[0];
  expect(getPendingAgentClarification).toHaveBeenCalledTimes(2);
  expect(resumed?.requestId).toBe(original.requestId);
  expect(resumed?.clarification).toEqual({ id, reply: 'На старый' });
  expect(resumed?.resume).toBe(true);
});
it('reload after an accepted reply retries the saved reply and original operation', async () => {
  const id = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  const requestId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
  vi.mocked(getPendingAgentClarification).mockResolvedValueOnce({
    input: {
      requestId,
      campaignId: 'campaign',
      sessionId: 'session',
      playerMessage: 'Иду на причал',
      history: [],
      clarification: { id, reply: 'На старый' },
    },
  });
  vi.mocked(streamAgentTurn).mockImplementationOnce(async (opts) => {
    opts.onEvent({ type: 'agent_done', totalRounds: 0 });
  });
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('На старый');
  });
  const resumed = vi.mocked(streamAgentTurn).mock.calls[0]?.[0];
  expect(resumed?.requestId).toBe(requestId);
  expect(resumed?.clarification).toEqual({ id, reply: 'На старый' });
  expect(resumed?.playerMessage).toBe('Иду на причал');
  expect(resumed?.resume).toBe(true);
});
it('keeps a clarification question and submits the reply to the same immutable operation', async () => {
  const clarificationId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  vi.mocked(streamAgentTurn)
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'text_delta', text: 'Какой причал вы имеете в виду?' });
      opts.onEvent({ type: 'agent_needs_input', clarificationId, commandId: 1, totalRounds: 1 });
    })
    .mockImplementationOnce(async (opts) => {
      opts.onEvent({ type: 'text_delta', text: 'Вы пришли на старый причал.' });
      opts.onEvent({ type: 'agent_done', totalRounds: 2 });
    });
  const { result } = renderHook(() => useAgentTurn());
  await act(async () => {
    await result.current.send('Иду на причал');
  });
  const first = vi.mocked(streamAgentTurn).mock.calls[0]?.[0];
  await act(async () => {
    await result.current.send('На старый');
  });
  const second = vi.mocked(streamAgentTurn).mock.calls[1]?.[0];
  expect(second?.requestId).toBe(first?.requestId);
  expect(second?.playerMessage).toBe('Иду на причал');
  expect(second?.history).toEqual(first?.history);
  expect(second?.resume).toBe(true);
  expect(second?.clarification).toEqual({ id: clarificationId, reply: 'На старый' });
  expect(useStore.getState().chat.messages.map((message) => message.content)).toEqual([
    'Иду на причал',
    'Какой причал вы имеете в виду?',
    'На старый',
    'Вы пришли на старый причал.',
  ]);
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
  await waitFor(() => expect(prepareBackendProvider).toHaveBeenCalled());
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
