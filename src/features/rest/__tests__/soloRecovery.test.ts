import { act, renderHook, waitFor } from '@testing-library/react';
import * as v from 'valibot';
import { beforeEach, expect, it, vi } from 'vitest';
import {
  SoloRecoveryRequestSchema,
  type SoloRecoveryResult,
  type SoloRecoveryStatus,
  SoloRecoveryStatusSchema,
} from '../../../api/contracts/soloRecovery';
import { ChatError } from '../../../api/errors';
import { getSoloRecoveryStatus, performSoloRecovery } from '../../../api/soloRecovery';
import { saveSoloRecoveryOutbox } from '../../../state/soloRecoveryOutbox';
import { useStore } from '../../../state/useStore';
import { useSoloRecovery } from '../useSoloRecovery';

vi.mock('../../../api/soloRecovery', () => ({
  getSoloRecoveryStatus: vi.fn(),
  performSoloRecovery: vi.fn(),
}));
const campaign = '11111111-1111-4111-8111-111111111111',
  session = '22222222-2222-4222-8222-222222222222',
  character = '33333333-3333-4333-8333-333333333333';
const status: SoloRecoveryStatus = {
  campaign_id: campaign,
  session_id: session,
  character_id: character,
  revision: 2,
  hp: 0,
  max_hp: 20,
  elapsed_seconds: 0,
  safe: true,
  combat_active: false,
  recovery: {
    character_id: character,
    status: 'dying',
    successes: 0,
    failures: 0,
    ready_at_seconds: null,
    actions: [{ kind: 'death_save' }],
  },
};
const result: SoloRecoveryResult = {
  status: {
    ...status,
    revision: 3,
    hp: 1,
    elapsed_seconds: 6,
    recovery: { ...status.recovery, status: 'healthy', actions: [] },
  },
  receipt: {
    receipt_revision: 3,
    duplicate: false,
    target_character_id: character,
    stabilized: false,
    recovered: true,
    ready_at_seconds: null,
    medicine: null,
    death_saves: [
      {
        character_id: character,
        result: {
          combatant: character,
          roll: 20,
          success: true,
          successes_total: 0,
          failures_total: 0,
          dead: false,
          stable: false,
        },
      },
    ],
  },
};
beforeEach(() => {
  vi.resetAllMocks();
  localStorage.clear();
  useStore.setState(useStore.getInitialState());
  useStore.getState().session.activate(campaign, session);
  useStore.setState((s) => ({
    pc: { ...s.pc, characterId: character, hp: 0, hpMax: 20 },
    campaigns: {
      ...s.campaigns,
      active: {
        id: campaign,
        last_session_id: session,
        revision: 2,
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        title: 'Solo',
        premise: 'Town',
        seed: { kind: 'collaborative', answers: { world: ['Town'] } },
        created_at: '2026-10-08T00:00:00Z',
        updated_at: '2026-10-08T00:00:00Z',
      },
    },
  }));
  vi.mocked(getSoloRecoveryStatus).mockResolvedValue(status);
});
it('reuses the same uncertain intention after remount and displays its recorded roll', async () => {
  vi.mocked(performSoloRecovery)
    .mockRejectedValueOnce(new ChatError('network', 'lost'))
    .mockResolvedValueOnce(result);
  const first = renderHook(() => useSoloRecovery());
  await waitFor(() => expect(first.result.current.status).not.toBeNull());
  await act(async () => {
    await first.result.current.perform({ kind: 'death_save' });
  });
  const original = vi.mocked(performSoloRecovery).mock.calls[0]?.[1];
  first.unmount();
  const second = renderHook(() => useSoloRecovery());
  await waitFor(() => expect(second.result.current.retryRequired).toBe(true));
  await act(async () => {
    await second.result.current.retry();
  });
  expect(vi.mocked(performSoloRecovery).mock.calls[1]?.[1]).toEqual(original);
  expect(useStore.getState().pc.hp).toBe(1);
  expect(second.result.current.receipt?.death_saves[0]?.result.roll).toBe(20);
  expect(useStore.getState().session.mutationId).toBeNull();
});
it('does not act for a different hero or on an intention the server did not offer', async () => {
  const hook = renderHook(() => useSoloRecovery());
  await waitFor(() => expect(hook.result.current.status).not.toBeNull());
  await act(async () => {
    expect(
      await hook.result.current.perform({ kind: 'wait', target_character_id: character }),
    ).toBe(false);
  });
  expect(performSoloRecovery).not.toHaveBeenCalled();
});
it('late recovery cannot overwrite a newly activated campaign', async () => {
  let finish!: (value: SoloRecoveryResult) => void;
  vi.mocked(performSoloRecovery).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const hook = renderHook(() => useSoloRecovery());
  await waitFor(() => expect(hook.result.current.status).not.toBeNull());
  let pending!: Promise<boolean>;
  act(() => {
    pending = hook.result.current.perform({ kind: 'death_save' });
  });
  useStore.getState().session.activate('another', 'session');
  useStore.setState((s) => ({ pc: { ...s.pc, hp: 5 } }));
  await act(async () => {
    finish(result);
    await pending;
  });
  expect(useStore.getState().pc.hp).toBe(5);
});
it('a delayed status cannot undo the confirmed recovery', async () => {
  let finish!: (value: SoloRecoveryStatus) => void;
  vi.mocked(getSoloRecoveryStatus).mockImplementation(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  saveSoloRecoveryOutbox(campaign, {
    session_id: session,
    character_id: character,
    expected_revision: 2,
    causation_id: '44444444-4444-4444-8444-444444444444',
    intent: { kind: 'death_save' },
  });
  vi.mocked(performSoloRecovery).mockResolvedValue(result);
  const hook = renderHook(() => useSoloRecovery());
  await act(async () => {
    await hook.result.current.retry();
  });
  await act(async () => {
    finish(status);
  });
  expect(hook.result.current.status?.revision).toBe(3);
  expect(useStore.getState().pc.hp).toBe(1);
});
it('rejects forged action associations and renderer dice', () => {
  expect(
    v.safeParse(SoloRecoveryStatusSchema, {
      ...status,
      recovery: { ...status.recovery, actions: [{ kind: 'aid', target_character_id: character }] },
    }).success,
  ).toBe(false);
  expect(v.safeParse(SoloRecoveryStatusSchema, { ...status, combat_active: true }).success).toBe(
    false,
  );
  expect(
    v.safeParse(SoloRecoveryRequestSchema, {
      session_id: session,
      character_id: character,
      expected_revision: 2,
      causation_id: campaign,
      intent: { kind: 'death_save', roll: 20 },
    }).success,
  ).toBe(false);
});
