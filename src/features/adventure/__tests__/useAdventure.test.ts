import { act, renderHook } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';

vi.mock('../../../api/agentPending', () => ({ getPendingAgentClarification: async () => null }));

import { streamAgentTurn } from '../../../api/agent';
import { actInAdventure } from '../../../api/campaigns';
import type { CampaignStateResponse } from '../../../api/contracts/campaignState';
import { ChatError } from '../../../api/errors';
import { useAgentTurn } from '../../../app/useAgentTurn';
import { buildCampaignState } from '../../../state/campaignProjection';
import { useStore } from '../../../state/useStore';
import { useAdventure } from '../useAdventure';

vi.mock('../../../api/agent', () => ({ streamAgentTurn: vi.fn() }));

vi.mock('../../../api/campaigns', () => ({ actInAdventure: vi.fn() }));
function state(
  campaign: 'A' | 'B',
  revision: number,
  location: 'town' | 'quay',
): CampaignStateResponse {
  const id =
    campaign === 'A'
      ? 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'
      : 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
  return {
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    character_id: 'cccccccc-cccc-4ccc-8ccc-cccccccccccc',
    session_id: id,
    campaign: {
      id,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: campaign,
      premise: '',
      seed: { kind: 'bundled', adventure_id: 'bell-beneath-water' },
      last_session_id: id,
      revision,
      created_at: '2026-10-06',
      updated_at: '2026-10-06',
    },
    projection: {
      messages: [],
      character: {
        name: campaign,
        heroClass: 'fighter',
        level: 1,
        hp: 12,
        hpMax: 12,
        inventory: [],
      },
      combat: null,
      scene: null,
      journal: [],
      npcs: [],
      adventure: {
        state: {
          package_id: 'bell-beneath-water',
          location,
          facts: ['ferry_route'],
          ending: null,
          closed: false,
          history: [],
        },
        available_actions: [{ kind: 'move', location: location === 'town' ? 'quay' : 'town' }],
      },
    },
  };
}
beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState(useStore.getInitialState());
  useStore.getState().restoration.apply(buildCampaignState(state('A', 2, 'town')));
});

it('blocks a new chat turn while an adventure projection is pending, even if its panel closes', async () => {
  let resolve: (value: CampaignStateResponse) => void = () => {};
  vi.mocked(actInAdventure).mockReturnValue(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const { result } = renderHook(() => ({ adventure: useAdventure(), agent: useAgentTurn() }));
  let pending: Promise<void> = Promise.resolve();
  act(() => {
    pending = result.current.adventure.perform({ kind: 'move', location: 'quay' });
  });
  await act(async () => {
    await result.current.agent.send('A new DM turn');
  });
  expect(streamAgentTurn).not.toHaveBeenCalled();
  expect(useStore.getState().chat.messages).toEqual([]);
  await act(async () => {
    resolve(state('A', 3, 'quay'));
    await pending;
  });
});
it('does not change the current scene before server confirmation', async () => {
  const visualEpoch = useStore.getState().session.presentationGeneration;
  let resolve: (value: CampaignStateResponse) => void = () => {};
  vi.mocked(actInAdventure).mockReturnValue(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const { result } = renderHook(() => useAdventure());
  let pending: Promise<void> = Promise.resolve();
  act(() => {
    pending = result.current.perform({ kind: 'move', location: 'quay' });
  });
  expect(useStore.getState().session.adventure?.state.location).toBe('town');
  await act(async () => {
    resolve(state('A', 3, 'quay'));
    await pending;
  });
  expect(useStore.getState().session.adventure?.state.location).toBe('quay');
  expect(useStore.getState().session.presentationGeneration).toBe(visualEpoch);
});
it('ignores a late result after selecting another campaign', async () => {
  let resolve: (value: CampaignStateResponse) => void = () => {};
  vi.mocked(actInAdventure).mockReturnValue(
    new Promise((r) => {
      resolve = r;
    }),
  );
  const { result } = renderHook(() => useAdventure());
  let pending: Promise<void> = Promise.resolve();
  act(() => {
    pending = result.current.perform({ kind: 'move', location: 'quay' });
  });
  act(() => {
    useStore.getState().restoration.apply(buildCampaignState(state('B', 2, 'town')));
  });
  await act(async () => {
    resolve(state('A', 3, 'quay'));
    await pending;
  });
  expect(useStore.getState().campaigns.active?.title).toBe('B');
  expect(useStore.getState().session.adventure?.state.location).toBe('town');
});
it('retries a lost response with the exact request and causation identity', async () => {
  vi.mocked(actInAdventure)
    .mockRejectedValueOnce(new ChatError('network', 'lost'))
    .mockResolvedValueOnce(state('A', 3, 'quay'));
  const { result } = renderHook(() => useAdventure());
  await act(async () => {
    await result.current.perform({ kind: 'move', location: 'quay' });
  });
  expect(result.current.retryRequired).toBe(true);
  await act(async () => {
    await result.current.retry();
  });
  expect(vi.mocked(actInAdventure).mock.calls[1]).toEqual(vi.mocked(actInAdventure).mock.calls[0]);
  expect(useStore.getState().campaigns.active?.revision).toBe(3);
});
