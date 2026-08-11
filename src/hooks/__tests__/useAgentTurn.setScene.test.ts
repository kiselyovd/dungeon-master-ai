import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentTurnOptions } from '../../api/agent';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

// Mock the SSE transport so the test can drive onToolCallResult directly.
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
});

describe('useAgentTurn set_scene handler', () => {
  it('updates the session scene when a set_scene tool result arrives', async () => {
    const { result } = renderHook(() => useAgentTurn());
    await result.current.send('explore the ruins');
    expect(useStore.getState().session.currentScene).toEqual({
      name: 'The Sunless Citadel',
      stepCounter: 0,
    });
  });
});
