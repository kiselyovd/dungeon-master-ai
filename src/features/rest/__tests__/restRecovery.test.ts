import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { RestStatus } from '../../../api/contracts/rest';
import { ChatError } from '../../../api/errors';
import { getRestStatus, performRest } from '../../../api/rest';
import { saveRestOutbox } from '../../../state/restOutbox';
import { useStore } from '../../../state/useStore';
import { useRest } from '../useRest';

vi.mock('../../../api/rest', () => ({ getRestStatus: vi.fn(), performRest: vi.fn() }));
const campaign = '11111111-1111-4111-8111-111111111111';
const session = '22222222-2222-4222-8222-222222222222';
const character = '33333333-3333-4333-8333-333333333333';
const status: RestStatus = {
  campaign_id: campaign,
  session_id: session,
  character_id: character,
  revision: 2,
  hp: 2,
  max_hp: 20,
  hit_die: 8,
  hit_dice_remaining: 1,
  slots: { 1: { maximum: 2, remaining: 0 } },
  elapsed_seconds: 0,
  long_rest_ready_in_seconds: 0,
  safe: true,
  combat_active: false,
};
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useStore.setState(useStore.getInitialState());
  useStore.getState().session.activate(campaign, session);
  useStore.setState((state) => ({
    pc: { ...state.pc, characterId: character, hp: 2, hpMax: 20 },
    campaigns: {
      ...state.campaigns,
      active: {
        id: campaign,
        last_session_id: session,
        revision: 2,
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        title: 'Rest',
        premise: 'Town',
        seed: { kind: 'collaborative', answers: { world: ['Town'] } },
        created_at: '2026-10-07T00:00:00Z',
        updated_at: '2026-10-07T00:00:00Z',
      },
    },
  }));
  vi.mocked(getRestStatus).mockResolvedValue(status);
});
it('retains an uncertain immutable request across dialog unmount and retries without new identity', async () => {
  vi.mocked(performRest)
    .mockRejectedValueOnce(new ChatError('network', 'lost'))
    .mockResolvedValueOnce({
      status: { ...status, revision: 3, hp: 8, hit_dice_remaining: 0, elapsed_seconds: 3600 },
      outcome: {
        hp: 8,
        elapsed_seconds: 3600,
        duration_minutes: 60,
        resources: { hit_dice_remaining: 0, last_long_rest_completed: null, spell_slots: { 1: 0 } },
        rolls: [],
      },
    });
  const first = renderHook(() => useRest());
  await waitFor(() => expect(first.result.current.status).not.toBeNull());
  await act(async () => {
    await first.result.current.perform({ kind: 'short', hit_dice: 1 });
  });
  const original = vi.mocked(performRest).mock.calls[0]?.[1];
  first.unmount();
  const second = renderHook(() => useRest());
  await waitFor(() => expect(second.result.current.retryRequired).toBe(true));
  await act(async () => {
    await second.result.current.retry();
  });
  expect(vi.mocked(performRest).mock.calls[1]?.[1]).toEqual(original);
  expect(useStore.getState().pc.hp).toBe(8);
  expect(useStore.getState().session.mutationId).toBeNull();
});
it('does not overwrite a different campaign when its response arrives late', async () => {
  let resolve!: (value: Awaited<ReturnType<typeof performRest>>) => void;
  vi.mocked(performRest).mockImplementation(
    () =>
      new Promise((r) => {
        resolve = r;
      }),
  );
  const hook = renderHook(() => useRest());
  await waitFor(() => expect(hook.result.current.status).not.toBeNull());
  let pending!: Promise<boolean>;
  act(() => {
    pending = hook.result.current.perform({ kind: 'long' });
  });
  useStore.getState().session.activate('another', 'session');
  useStore.setState((state) => ({ pc: { ...state.pc, hp: 5 } }));
  await act(async () => {
    resolve({
      status: { ...status, revision: 3, hp: 20, elapsed_seconds: 28800 },
      outcome: {
        hp: 20,
        elapsed_seconds: 28800,
        duration_minutes: 480,
        resources: {
          hit_dice_remaining: 1,
          last_long_rest_completed: 28800,
          spell_slots: { 1: 2 },
        },
        rolls: [],
      },
    });
    await pending;
  });
  expect(useStore.getState().pc.hp).toBe(5);
});

it('a delayed status read cannot undo a successfully recovered rest', async () => {
  let resolveStatus!: (value: RestStatus) => void;
  vi.mocked(getRestStatus).mockImplementation(
    () =>
      new Promise((resolve) => {
        resolveStatus = resolve;
      }),
  );
  saveRestOutbox(campaign, {
    session_id: session,
    character_id: character,
    expected_revision: 2,
    causation_id: '44444444-4444-4444-8444-444444444444',
    intent: { kind: 'short', hit_dice: 1 },
  });
  vi.mocked(performRest).mockResolvedValue({
    status: { ...status, revision: 3, hp: 8, hit_dice_remaining: 0, elapsed_seconds: 3600 },
    outcome: {
      hp: 8,
      elapsed_seconds: 3600,
      duration_minutes: 60,
      resources: { hit_dice_remaining: 0, last_long_rest_completed: null, spell_slots: { 1: 0 } },
      rolls: [],
    },
  });
  const hook = renderHook(() => useRest());
  await act(async () => {
    await hook.result.current.retry();
  });
  await act(async () => {
    resolveStatus(status);
  });
  expect(hook.result.current.status?.revision).toBe(3);
  expect(hook.result.current.status?.hp).toBe(8);
});
