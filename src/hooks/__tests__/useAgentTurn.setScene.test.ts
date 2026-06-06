import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AgentTurnOptions } from '../../api/agent';
import { useStore } from '../../state/useStore';
import { useAgentTurn } from '../useAgentTurn';

// Mock the SSE transport so the test can drive onToolCallResult directly.
vi.mock('../../api/agent', () => ({
  streamAgentTurn: vi.fn(async (opts: AgentTurnOptions) => {
    opts.onToolCallStart('tc-1', 'set_scene', 1);
    opts.onToolCallResult(
      'tc-1',
      'set_scene',
      { title: 'The Sunless Citadel', mode: 'exploration' },
      { scene_id: 's-1' },
      false,
      1,
      'engine',
    );
    opts.onAgentDone(1);
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
