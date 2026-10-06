import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { commitCampaignCharacter, fetchCampaignState } from '../../../api/campaigns';
import type { CampaignStateResponse } from '../../../api/contracts/campaignState';
import { useStore } from '../../../state/useStore';
import { useCampaigns } from '../useCampaigns';

vi.mock('../../../api/campaigns', () => ({
  fetchCampaignState: vi.fn(),
  fetchCampaign: vi.fn(async (id: string) => (await fetchCampaignState(id)).campaign),
  fetchCampaigns: vi.fn(async () => []),
  createCampaign: vi.fn(),
  commitCampaignCharacter: vi.fn(),
}));

function state(id: string): CampaignStateResponse {
  return {
    schema_version: 1,
    ruleset_id: 'srd-5.2.1',
    campaign: {
      id,
      schema_version: 1,
      ruleset_id: 'srd-5.2.1',
      title: id,
      premise: '',
      seed: { kind: 'collaborative', answers: {} },
      last_session_id: `session-${id}`,
      revision: 1,
      created_at: '2026-10-06T00:00:00Z',
      updated_at: '2026-10-06T00:00:00Z',
    },
    session_id: `session-${id}`,
    character_id: null,
    projection: { character: {}, messages: [], combat: null, scene: null, journal: [], npcs: [] },
  };
}

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.clearAllMocks();
});

describe('campaign transitions', () => {
  it('retains the unfinished wizard draft only when resuming its exact active session', async () => {
    useStore.getState().session.activate('A', 'session-A');
    useStore.getState().charCreation.setDraftField('classId', 'wizard');
    useStore.getState().charCreation.setDraftField('name', 'Unfinished Mira');
    vi.mocked(fetchCampaignState).mockResolvedValue(state('A'));
    const { result } = renderHook(() => useCampaigns());
    await act(async () => {
      await result.current.activate('A');
    });
    expect(useStore.getState().charCreation.name).toBe('Unfinished Mira');
    vi.mocked(fetchCampaignState).mockResolvedValue(state('B'));
    await act(async () => {
      await result.current.activate('B');
    });
    expect(useStore.getState().charCreation.name).toBe('');
  });
  it('keeps the legacy local hero when its exact active campaign migration fails', async () => {
    useStore.getState().pc.applyPreset('fighter');
    useStore.getState().session.activate('A', 'session-A');
    vi.mocked(fetchCampaignState).mockResolvedValue(state('A'));
    vi.mocked(commitCampaignCharacter).mockRejectedValue(new Error('offline'));
    const { result } = renderHook(() => useCampaigns());
    await act(async () => {
      await result.current.activate('A');
    });
    expect(useStore.getState().pc.heroClass).toBe('fighter');
    expect(useStore.getState().session.activeCampaignId).toBe('A');
    expect(useStore.getState().campaigns.errorCode).not.toBeNull();
  });
  it('keeps the newest activation when an older response finishes late', async () => {
    let resolveA: (value: CampaignStateResponse) => void = () => {};
    const pendingA = new Promise<CampaignStateResponse>((resolve) => {
      resolveA = resolve;
    });
    vi.mocked(fetchCampaignState).mockImplementation(async (id) =>
      id === 'A' ? pendingA : state(id),
    );
    const { result } = renderHook(() => useCampaigns());
    let activationA: Promise<unknown> = Promise.resolve();
    act(() => {
      activationA = result.current.activate('A');
    });
    await act(async () => {
      await result.current.activate('B');
    });
    await act(async () => {
      resolveA(state('A'));
      await activationA;
    });
    expect(useStore.getState().campaigns.active?.id).toBe('B');
    expect(useStore.getState().session.activeSessionId).toBe('session-B');
  });

  it('clears the former hero and history when opening an empty campaign', async () => {
    useStore.getState().pc.applyPreset('fighter');
    useStore.getState().chat.appendUser('Old story');
    vi.mocked(fetchCampaignState).mockResolvedValue(state('B'));
    const { result } = renderHook(() => useCampaigns());
    await act(async () => {
      await result.current.activate('B');
    });
    expect(useStore.getState().pc.heroClass).toBeNull();
    expect(useStore.getState().chat.messages).toEqual([]);
    expect(useStore.getState().campaigns.active?.id).toBe('B');
  });

  it('rejects a state for another campaign without partially changing the active campaign', async () => {
    useStore.getState().campaigns.setActive(state('A').campaign);
    useStore.getState().session.activate('A', 'session-A');
    vi.mocked(fetchCampaignState).mockResolvedValue(state('C'));
    const { result } = renderHook(() => useCampaigns());
    await act(async () => {
      await result.current.activate('B');
    });
    expect(useStore.getState().campaigns.active?.id).toBe('A');
    expect(useStore.getState().session.activeSessionId).toBe('session-A');
    expect(useStore.getState().campaigns.errorCode).not.toBeNull();
  });
});
