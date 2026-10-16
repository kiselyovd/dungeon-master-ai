import { afterEach, expect, it, vi } from 'vitest';
import { streamAgentTurn } from '../agent';
import { setBackendPortForTesting } from '../client';

afterEach(() => {
  setBackendPortForTesting(null);
  vi.unstubAllGlobals();
});

it('surfaces the safe turn failure without treating it as completion', async () => {
  setBackendPortForTesting(45678);
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue(
      new Response('event: error\ndata: {"code":"agent_turn_failed"}\n\n', {
        status: 200,
        headers: { 'content-type': 'text/event-stream' },
      }),
    ),
  );
  const onEvent = vi.fn();
  await expect(
    streamAgentTurn({
      campaignId: 'campaign',
      sessionId: 'session',
      playerMessage: 'Inspect',
      history: [],
      onEvent,
    }),
  ).rejects.toMatchObject({ code: 'agent_turn_failed' });
  expect(onEvent).not.toHaveBeenCalled();
});
