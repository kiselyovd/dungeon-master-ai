import { act, renderHook, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { AgentRecoveryProjection } from '../../api/contracts/agentRecovery';
import { type RecoveryPorts, type RecoveryScope, useRecovery } from './useRecovery';

const scope: RecoveryScope = {
  campaignId: 'campaign-a',
  sessionId: 'session-a',
  actorId: 'hero-a',
  generation: 1,
};
function ports(pending = true): RecoveryPorts {
  const projection = {
    schema_version: 1,
    scope: {
      request_id: 'request-a',
      campaign_id: scope.campaignId,
      session_id: scope.sessionId,
      actor_id: scope.actorId,
      authority: { kind: 'solo', restore_generation: 0 },
    },
    model: 'model-a',
    phase: 'narration_pending',
    allowed_action: 'resume',
    progress_known: true,
    commands: [{ command_id: 1, state: 'finished' }],
    confirmed_effects: [],
    clarification: null,
  } as AgentRecoveryProjection;
  return {
    projection: vi.fn(async () => (pending ? projection : null)),
    input: vi.fn(async () =>
      pending
        ? {
            input: {
              requestId: 'request-a',
              campaignId: scope.campaignId,
              sessionId: scope.sessionId,
              playerMessage: 'Original action',
              history: [],
              model: 'model-a',
            },
          }
        : null,
    ),
    prepare: vi.fn(() => true),
    resume: vi.fn(async () => {}),
  };
}
it('discovers the original request before another send and resumes its identity', async () => {
  const transport = ports();
  const { result } = renderHook(() => useRecovery(scope, transport, false));
  await waitFor(() => expect(result.current.status).toBe('ready'));
  expect(result.current.blocked).toBe(true);
  expect(result.current.view?.requestId).toBe('request-a');
  await act(() => result.current.resume());
  expect(transport.resume).toHaveBeenCalledWith(
    expect.objectContaining({
      requestId: 'request-a',
      playerMessage: 'Original action',
      model: 'model-a',
    }),
    undefined,
  );
});
it('keeps an unreadable pending state blocked without losing scope', async () => {
  const transport = ports();
  transport.input = vi.fn(async () => {
    throw new Error('offline');
  });
  const { result } = renderHook(() => useRecovery(scope, transport, false));
  await waitFor(() => expect(result.current.status).toBe('error'));
  expect(result.current.blocked).toBe(true);
  await act(() => result.current.resume());
  expect(transport.resume).not.toHaveBeenCalled();
});
it('retries an inconsistent snapshot instead of inventing an empty pending slot', async () => {
  const transport = ports();
  transport.input = vi.fn(async () => null);
  const { result } = renderHook(() => useRecovery(scope, transport, false));
  await waitFor(() => expect(result.current.status).toBe('error'));
  expect(transport.projection).toHaveBeenCalledTimes(2);
  expect(result.current.blocked).toBe(true);
});
it('ignores a delayed previous campaign and releases only an explicit empty read', async () => {
  const transport = ports(false);
  let resolveOld: (value: AgentRecoveryProjection | null) => void = () => {};
  transport.projection = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveOld = resolve;
        }),
    )
    .mockResolvedValue(null);
  const { result, rerender } = renderHook(({ active }) => useRecovery(active, transport, false), {
    initialProps: { active: scope },
  });
  rerender({
    active: { ...scope, campaignId: 'campaign-b', sessionId: 'session-b', generation: 2 },
  });
  await waitFor(() => expect(result.current.status).toBe('ready'));
  await act(async () => resolveOld(null));
  expect(result.current.view).toBeNull();
  expect(result.current.blocked).toBe(false);
  expect(transport.prepare).not.toHaveBeenCalled();
});
