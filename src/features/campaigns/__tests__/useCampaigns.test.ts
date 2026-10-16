import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  commitCampaignCharacter,
  createCampaign,
  fetchCampaignState,
} from '../../../api/campaigns';
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
  it('reuses the durable creation request after an unknown response and component remount', async () => {
    const id = 'cccccccc-cccc-4ccc-8ccc-cccccccccccc';
    const canonical = state(id);
    canonical.session_id = 'dddddddd-dddd-4ddd-8ddd-dddddddddddd';
    canonical.campaign.last_session_id = canonical.session_id;
    const input = {
      title: `Road-${crypto.randomUUID()}`,
      premise: 'Rain',
      seed: { kind: 'collaborative' as const, answers: {} },
    };
    vi.mocked(createCampaign)
      .mockRejectedValueOnce(new Error('unknown ACK'))
      .mockResolvedValue(canonical.campaign);
    vi.mocked(fetchCampaignState).mockResolvedValue(canonical);
    const first = renderHook(() => useCampaigns());
    await act(async () => {
      expect(await first.result.current.create(input)).toBeNull();
    });
    const original = vi.mocked(createCampaign).mock.calls[0]?.[0];
    if (!original) throw new Error('creation request fixture missing');
    first.unmount();
    const cold = renderHook(() => useCampaigns());
    await act(async () => {
      expect(await cold.result.current.create(input)).toEqual(canonical.campaign);
    });
    expect(vi.mocked(createCampaign).mock.calls[1]?.[0]).toEqual(original);
    expect(original.requestId).toMatch(/^[0-9a-f-]{36}$/);
  });
  it('preserves a campaign-scoped hero draft when backing out to the library and reopening', async () => {
    useStore
      .getState()
      .charCreation.bindScope({ kind: 'campaign', campaignId: 'A', characterId: null });
    useStore.getState().charCreation.setDraftField('name', 'Mira draft');
    const hook = renderHook(() => useCampaigns());
    await act(async () => {
      await hook.result.current.clear(true);
    });
    vi.mocked(fetchCampaignState).mockResolvedValue(state('A'));
    await act(async () => {
      await hook.result.current.activate('A');
    });
    expect(useStore.getState().charCreation.name).toBe('Mira draft');
    expect(useStore.getState().pc.heroClass).toBeNull();
  });
  it('records visits only for the latest successful activation', async () => {
    const first = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
    const latest = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
    let finishFirst: (value: CampaignStateResponse) => void = () => {};
    const firstResponse = new Promise<CampaignStateResponse>((resolve) => {
      finishFirst = resolve;
    });
    vi.mocked(fetchCampaignState).mockImplementation(async (id) =>
      id === first ? firstResponse : state(id),
    );
    const { result } = renderHook(() => useCampaigns());
    let older: Promise<unknown> = Promise.resolve();
    act(() => {
      older = result.current.activate(first);
    });
    await act(async () => {
      await result.current.activate(latest);
    });
    await act(async () => {
      finishFirst(state(first));
      await older;
    });
    const visits = useStore.getState().settings.campaignVisits;
    expect(visits[first]).toBeUndefined();
    expect(visits[latest]).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(useStore.getState().campaigns.active?.revision).toBe(1);
    vi.mocked(fetchCampaignState).mockRejectedValue(new Error('fixture failed read'));
    await act(async () => {
      await result.current.activate(first);
    });
    expect(useStore.getState().settings.campaignVisits[first]).toBeUndefined();
  });
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
