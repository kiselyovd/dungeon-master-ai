import { afterEach, expect, it, vi } from 'vitest';
import { streamAgentTurn } from './agent';
import { setBackendPortForTesting } from './client';

afterEach(() => {
  vi.unstubAllGlobals();
  setBackendPortForTesting(null);
});
it('accepts a validated needs-input terminal without claiming the action completed', async () => {
  setBackendPortForTesting(31415);
  const clarificationId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(
          `event: text_delta\ndata: {"text":"Кого атаковать?"}\n\nevent: agent_needs_input\ndata: {"clarification_id":"${clarificationId}","command_id":1,"total_rounds":1}\n\n`,
          { headers: { 'content-type': 'text/event-stream' } },
        ),
    ),
  );
  const onEvent = vi.fn();
  await streamAgentTurn({
    campaignId: 'campaign',
    sessionId: 'session',
    playerMessage: 'Атакую врага',
    history: [],
    onEvent,
  });
  expect(onEvent.mock.calls.map(([event]) => event.type)).toEqual([
    'text_delta',
    'agent_needs_input',
  ]);
});
it('sends a supplied stable operation ID and rejects EOF without a terminal marker', async () => {
  setBackendPortForTesting(31415);
  const fetch = vi.fn<typeof globalThis.fetch>(
    async () =>
      new Response('event: text_delta\ndata: {"text":"Partial reply"}\n\n', {
        status: 200,
        headers: { 'content-type': 'text/event-stream' },
      }),
  );
  vi.stubGlobal('fetch', fetch);
  const requestId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
  await expect(
    streamAgentTurn({
      requestId,
      campaignId: 'campaign',
      sessionId: 'session',
      playerMessage: 'Look',
      history: [],
      onEvent: vi.fn(),
    }),
  ).rejects.toMatchObject({ code: 'network' });
  const init = fetch.mock.calls[0]?.[1] as RequestInit;
  expect(JSON.parse(init.body as string).request_id).toBe(requestId);
});
