import { afterEach, expect, it, vi } from 'vitest';
import { getPendingAgentClarification } from './agentPending';
import { setBackendPortForTesting } from './client';

const campaignId = '11111111-1111-4111-8111-111111111111';
const sessionId = '22222222-2222-4222-8222-222222222222';
afterEach(() => {
  vi.unstubAllGlobals();
  setBackendPortForTesting(null);
});
function wire() {
  return {
    operation: {
      request_id: '33333333-3333-4333-8333-333333333333',
      model: null,
      request: {
        campaign_id: campaignId,
        session_id: sessionId,
        player_message: 'Иду на причал',
        history: [{ role: 'user', parts: [] }],
        images: [],
        board: null,
      },
    },
    clarification: {
      id: '44444444-4444-4444-8444-444444444444',
      command_id: 1,
      question: 'Какой причал?',
      reply: null,
    },
  };
}
it('recovers exact original wire history including an empty multipart message', async () => {
  setBackendPortForTesting(31415);
  vi.stubGlobal(
    'fetch',
    vi.fn(
      async () =>
        new Response(JSON.stringify(wire()), { headers: { 'content-type': 'application/json' } }),
    ),
  );
  const pending = await getPendingAgentClarification(campaignId, sessionId);
  expect(pending?.input.wireHistory).toEqual([{ role: 'user', parts: [] }]);
  expect(pending?.input.playerMessage).toBe('Иду на причал');
});
it('rejects another session rather than treating it as a new command', async () => {
  setBackendPortForTesting(31415);
  const wrong = wire();
  wrong.operation.request.session_id = campaignId;
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(JSON.stringify(wrong))),
  );
  await expect(getPendingAgentClarification(campaignId, sessionId)).rejects.toMatchObject({
    code: 'invalid_response',
  });
});
it('recovers an accepted reply without turning it into a new answer', async () => {
  setBackendPortForTesting(31415);
  const saved = wire();
  const body = { ...saved, clarification: { ...saved.clarification, reply: 'На старый' } };
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => new Response(JSON.stringify(body))),
  );
  const pending = await getPendingAgentClarification(campaignId, sessionId);
  expect(pending?.clarificationId).toBeUndefined();
  expect(pending?.input.clarification).toEqual({ id: saved.clarification.id, reply: 'На старый' });
});
