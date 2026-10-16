import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../api/agentPending', () => ({ getPendingAgentClarification: async () => null }));

import { type AgentTurnOptions, streamAgentTurn } from '../../api/agent';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

// Mock the SSE transport so the test can drive onToolCallResult directly.
vi.mock('../../app/providerStartup', () => ({ prepareBackendProvider: vi.fn(async () => {}) }));
vi.mock('../../api/agent', () => ({
  streamAgentTurn: vi.fn(async (opts: AgentTurnOptions) => {
    opts.onEvent?.({ type: 'tool_call_start', id: 'tc-1', toolName: 'set_scene', round: 1 });
    opts.onEvent?.({
      type: 'tool_call_result',
      id: 'tc-1',
      toolName: 'set_scene',
      args: { title: 'The Sunless Citadel', mode: 'exploration' },
      result: { scene_id: 's-1' },
      isError: false,
      round: 1,
      handledBy: 'engine',
    });
    opts.onEvent?.({ type: 'agent_done', totalRounds: 1 });
  }),
}));

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.mocked(streamAgentTurn).mockClear();
});

describe('useAgentTurn set_scene handler', () => {
  it('fails closed when no campaign is active', async () => {
    const { result } = renderHook(() => useAgentTurn());
    await result.current.send('explore the ruins');
    expect(streamAgentTurn).not.toHaveBeenCalled();
    expect(useStore.getState().chat.lastError).toMatchObject({ code: 'campaign_required' });
  });

  it('updates the session scene when a set_scene tool result arrives', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    const { result } = renderHook(() => useAgentTurn());
    await result.current.send('explore the ruins');
    expect(useStore.getState().session.currentScene).toEqual({
      name: 'The Sunless Citadel',
      stepCounter: 0,
    });
  });

  it('ignores late events and finalization from a campaign that was left', async () => {
    let finish: () => void = () => {};
    const pending = new Promise<void>((resolve) => {
      finish = resolve;
    });
    vi.mocked(streamAgentTurn).mockImplementationOnce(async (opts) => {
      await pending;
      opts.onEvent?.({ type: 'text_delta', text: 'Old campaign reply' });
      opts.onEvent?.({ type: 'agent_done', totalRounds: 1 });
    });
    useStore.getState().session.activate('campaign-1', 'session-1');
    const { result } = renderHook(() => useAgentTurn());
    let turn: Promise<void> = Promise.resolve();
    act(() => {
      turn = result.current.send('old action');
    });
    await waitFor(() => expect(streamAgentTurn).toHaveBeenCalledTimes(1));
    act(() => {
      useStore.getState().restoration.clear();
    });
    await act(async () => {
      finish();
      await turn;
    });
    expect(useStore.getState().chat.messages).toEqual([]);
    expect(useStore.getState().chat.lastError).toBeNull();
  });
});
