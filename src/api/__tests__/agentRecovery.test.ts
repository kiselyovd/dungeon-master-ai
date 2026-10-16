import { afterEach, expect, it, vi } from 'vitest';
import { getPendingAgentRecoveryInput } from '../agentPending';
import { fetchAgentRecovery } from '../agentRecovery';
import { parseAgentRecovery } from '../contracts/agentRecovery';

vi.mock('../client', () => ({
  backendUrl: async (path: string) => `http://localhost:31415${path}`,
}));
const campaignId = '11111111-1111-4111-8111-111111111111';
const sessionId = '22222222-2222-4222-8222-222222222222';
function projection() {
  return {
    schema_version: 1,
    scope: {
      request_id: '33333333-3333-4333-8333-333333333333',
      campaign_id: campaignId,
      session_id: sessionId,
      actor_id: '44444444-4444-4444-8444-444444444444',
      authority: { kind: 'solo', restore_generation: 0 },
    },
    model: 'model-a',
    phase: 'narration_pending',
    allowed_action: 'resume',
    progress_known: true,
    commands: [{ command_id: 1, state: 'finished' }],
    confirmed_effects: [
      {
        command_id: 1,
        step: 1,
        action: 'adventure',
        outcome: { kind: 'performed', check_success: null },
      },
    ],
    clarification: null,
  };
}
afterEach(() => vi.unstubAllGlobals());
it('hydrates the original manifest-free request through the separate owner endpoint', async () => {
  const input = {
    operation: {
      request_id: projection().scope.request_id,
      model: 'model-a',
      request: {
        campaign_id: campaignId,
        session_id: sessionId,
        player_message: 'Original action',
        history: [],
        images: [],
        board: null,
      },
    },
    clarification: null,
  };
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(input), { status: 200 }));
  vi.stubGlobal('fetch', fetch);
  const recovered = await getPendingAgentRecoveryInput(campaignId, sessionId);
  expect(recovered?.input.requestId).toBe(input.operation.request_id);
  expect(recovered?.input.playerMessage).toBe('Original action');
  expect(recovered?.input.model).toBe('model-a');
  expect(fetch.mock.calls[0]?.[0]).toContain('/agent/recovery-input?');
});
it('keeps an unavailable recovery-input endpoint distinct from an empty pending slot', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 404 })));
  await expect(getPendingAgentRecoveryInput(campaignId, sessionId)).rejects.toMatchObject({
    code: 'http_error',
  });
});

it('parses confirmed progress with original request and model identity', async () => {
  const value = projection();
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify(value), { status: 200 }));
  vi.stubGlobal('fetch', fetch);
  const result = await fetchAgentRecovery(campaignId, sessionId);
  expect(result).toEqual(value);
  expect(fetch.mock.calls[0]?.[0]).toContain('/agent/recovery?campaign_id=');
});
it('accepts only explicit successful null as no pending recovery', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('null', { status: 200 })));
  expect(await fetchAgentRecovery(campaignId, sessionId)).toBeNull();
});
it('does not treat a missing old-backend endpoint as no pending request', async () => {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response('', { status: 404 })));
  await expect(fetchAgentRecovery(campaignId, sessionId)).rejects.toMatchObject({
    code: 'http_error',
  });
});
it('rejects a foreign campaign without applying its recovery', async () => {
  const value = projection();
  value.scope.campaign_id = '55555555-5555-4555-8555-555555555555';
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue(new Response(JSON.stringify(value), { status: 200 })),
  );
  await expect(fetchAgentRecovery(campaignId, sessionId)).rejects.toMatchObject({
    code: 'invalid_response',
  });
});
it.each([
  'history',
  'reasoning',
  'manifest',
  'capability',
])('rejects private extra field %s', (field) => {
  expect(() => parseAgentRecovery({ ...projection(), [field]: 'PRIVATE' })).toThrow(
    'agent_recovery_invalid',
  );
});
it('does not accept contradictory unknown progress or duplicate command identities', () => {
  expect(() => parseAgentRecovery({ ...projection(), progress_known: false })).toThrow();
  const value = projection();
  value.commands.push({ command_id: 1, state: 'finished' });
  expect(() => parseAgentRecovery(value)).toThrow();
});
it('keeps a running operation gated and bounds its pending result', () => {
  const value = { ...projection(), phase: 'running', allowed_action: 'wait' };
  expect(parseAgentRecovery(value)?.allowed_action).toBe('wait');
  expect(() => parseAgentRecovery({ ...value, allowed_action: 'resume' })).toThrow();
  expect(() =>
    parseAgentRecovery({
      ...projection(),
      confirmed_effects: Array(1025).fill(projection().confirmed_effects[0]),
    }),
  ).toThrow();
});
