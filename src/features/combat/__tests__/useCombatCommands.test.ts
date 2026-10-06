import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useStore } from '../../../state/useStore';
import { sendCombatCommand } from '../model/commands';
import type { CombatProjectionDto } from '../model/types';
import { useCombatCommands } from '../useCombatCommands';

vi.mock('../model/commands', async (importOriginal) => {
  const original = await importOriginal<typeof import('../model/commands')>();
  return { ...original, sendCombatCommand: vi.fn() };
});

function projection(revision: number, x: number): CombatProjectionDto {
  return {
    schema_version: 1,
    encounter_id: 'enc',
    revision,
    events: [],
    snapshot: {
      active: true,
      round: 1,
      current_combatant: 'hero',
      initiative: [{ id: 'hero', roll: 20, dex_tiebreak: 2 }],
      combatants: [
        {
          id: 'hero',
          name: 'Hero',
          max_hp: 10,
          current_hp: 10,
          temp_hp: 0,
          ac: 15,
          speed_ft: 30,
          initiative_roll: 20,
          dex_mod: 2,
          conditions: [],
          budget: { action: true, bonus_action: true, reaction: true, movement_ft: 30 },
          is_dead: false,
          position: { x, y: 0 },
        },
      ],
    },
  };
}

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  useStore.getState().combat.replaceProjection(projection(0, 0));
  vi.mocked(sendCombatCommand).mockReset();
});

describe('useCombatCommands', () => {
  it('does not send a player keyboard intention for an unowned same-name actor', async () => {
    useStore.getState().pc.replaceFromDraft({ characterId: 'owned-character', name: 'Hero' });
    vi.mocked(sendCombatCommand).mockResolvedValue(projection(1, 2));
    const { result } = renderHook(() => useCombatCommands());
    await act(async () => result.current.endTurn());
    await act(async () => result.current.move('hero', 2, 0));
    expect(sendCombatCommand).not.toHaveBeenCalled();
  });
  it('retries an unchanged failed intention with its original UUID and revision', async () => {
    vi.mocked(sendCombatCommand)
      .mockRejectedValueOnce(new Error('connection lost'))
      .mockResolvedValueOnce(projection(1, 2));
    const { result } = renderHook(() => useCombatCommands());
    await act(async () => result.current.move('hero', 2, 0));
    await act(async () => result.current.move('hero', 2, 0));
    const calls = vi.mocked(sendCombatCommand).mock.calls;
    expect(calls).toHaveLength(2);
    expect(calls[1]?.[0]).toEqual(calls[0]?.[0]);
    expect(useStore.getState().combat.tokens[0]?.x).toBe(2);
  });
  it('uses UUID command identities that remain unique when a hook is mounted again', async () => {
    vi.mocked(sendCombatCommand).mockResolvedValue(projection(1, 1));
    const first = renderHook(() => useCombatCommands());
    await act(async () => first.result.current.move('hero', 1, 0));
    first.unmount();
    const second = renderHook(() => useCombatCommands());
    vi.mocked(sendCombatCommand).mockResolvedValue(projection(2, 2));
    await act(async () => second.result.current.move('hero', 2, 0));
    const ids = vi.mocked(sendCombatCommand).mock.calls.map(([input]) => input.commandId);
    expect(new Set(ids).size).toBe(2);
    for (const id of ids)
      expect(id).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  });
  it('does not resurrect a combat response from before a campaign transition', async () => {
    let resolve!: (value: CombatProjectionDto) => void;
    vi.mocked(sendCombatCommand).mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const { result } = renderHook(() => useCombatCommands());
    let request!: Promise<void>;
    act(() => {
      request = result.current.move('hero', 2, 0) as Promise<void>;
    });
    act(() => {
      useStore.getState().session.beginTransition();
      useStore.getState().combat.clearProjection();
    });
    resolve(projection(1, 2));
    await act(async () => request);
    expect(useStore.getState().combat.encounterId).toBeNull();
    expect(useStore.getState().combat.tokens).toEqual([]);
  });
  it('keeps movement unchanged while pending and reconciles only from the response projection', async () => {
    let resolve!: (value: CombatProjectionDto) => void;
    vi.mocked(sendCombatCommand).mockImplementation(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const { result } = renderHook(() => useCombatCommands());
    let request!: Promise<void>;
    act(() => {
      request = result.current.move('hero', 2, 0) as Promise<void>;
    });
    expect(useStore.getState().combat.tokens[0]?.x).toBe(0);
    expect(Object.keys(useStore.getState().combat.pendingCommands)).toHaveLength(1);
    resolve(projection(1, 2));
    await act(async () => request);
    expect(useStore.getState().combat.tokens[0]?.x).toBe(2);
    expect(useStore.getState().combat.pendingCommands).toEqual({});
  });

  it('leaves projection unchanged when the command is rejected', async () => {
    vi.mocked(sendCombatCommand).mockRejectedValue(new Error('rejected'));
    const { result } = renderHook(() => useCombatCommands());
    await act(async () => result.current.move('hero', 3, 0));
    expect(useStore.getState().combat.tokens[0]?.x).toBe(0);
    expect(useStore.getState().combat.revision).toBe(0);
  });
});
