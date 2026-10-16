import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fetchSessionMessages } from '../../api/chat';
import type { ChatMessage } from '../../state/chat';
import { useStore } from '../../state/useStore';
import { useSession } from '../useSession';

vi.mock('../../api/chat', () => ({
  fetchSessionMessages: vi.fn(),
}));

const fetchMock = vi.mocked(fetchSessionMessages);

describe('useSession', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    fetchMock.mockReset();
  });

  it('fails closed without minting IDs when no campaign is active', async () => {
    renderHook(() => useSession());

    await waitFor(() => {
      expect(useStore.getState().session.loadError).toBe('campaign_required');
    });
    expect(fetchMock).not.toHaveBeenCalled();
    expect(useStore.getState().session.activeCampaignId).toBeNull();
    expect(useStore.getState().session.activeSessionId).toBeNull();
  });

  it('seeds the chat slice with persisted history when the backend returns messages', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    const persisted: ChatMessage[] = [
      { id: 'a', role: 'user', content: 'hello' },
      { id: 'b', role: 'assistant', content: 'world' },
    ];
    fetchMock.mockResolvedValue(persisted);

    renderHook(() => useSession());

    await waitFor(() => {
      expect(useStore.getState().chat.messages).toHaveLength(2);
    });
    expect(useStore.getState().chat.messages[0]).toMatchObject({ role: 'user', content: 'hello' });
    expect(useStore.getState().chat.messages[1]).toMatchObject({
      role: 'assistant',
      content: 'world',
    });
  });

  it('leaves the chat empty when the backend fetch rejects', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    fetchMock.mockRejectedValue(new Error('backend down'));

    renderHook(() => useSession());

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('records the load error on session.loadError when the fetch rejects', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    fetchMock.mockRejectedValue(new Error('backend down'));

    renderHook(() => useSession());

    await waitFor(() => {
      expect(useStore.getState().session.loadError).toBe('backend down');
    });
  });

  it('refetch retries the fetch and clears loadError on success', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    fetchMock.mockRejectedValueOnce(new Error('backend down'));

    const { result } = renderHook(() => useSession());

    await waitFor(() => {
      expect(useStore.getState().session.loadError).toBe('backend down');
    });

    fetchMock.mockResolvedValueOnce([{ id: 'a', role: 'user', content: 'after retry' }]);

    result.current.refetch();

    await waitFor(() => {
      expect(useStore.getState().session.loadError).toBeNull();
    });
    expect(useStore.getState().chat.messages).toEqual([
      { id: 'a', role: 'user', content: 'after retry' },
    ]);
  });

  it('clears stale chat when the backend confirms that the active session is empty', async () => {
    useStore.getState().session.activate('campaign-1', 'session-1');
    useStore.setState((s) => ({
      chat: {
        ...s.chat,
        messages: [{ id: 'pre-existing', role: 'user', content: 'staged' }],
      },
    }));
    fetchMock.mockResolvedValue([]);

    renderHook(() => useSession());

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('reloads on session change and ignores late history from the old session', async () => {
    let resolveOld: (messages: ChatMessage[]) => void = () => {};
    const pending = new Promise<ChatMessage[]>((resolve) => {
      resolveOld = resolve;
    });
    fetchMock.mockImplementation(async (id) =>
      id === 'session-A' ? pending : [{ id: 'B', role: 'assistant', content: 'New story' }],
    );
    useStore.getState().session.activate('A', 'session-A');
    renderHook(() => useSession());
    act(() => useStore.getState().session.activate('B', 'session-B'));
    await waitFor(() => expect(useStore.getState().chat.messages[0]?.content).toBe('New story'));
    await act(async () => {
      resolveOld([{ id: 'A', role: 'assistant', content: 'Old story' }]);
      await pending;
    });
    expect(useStore.getState().chat.messages[0]?.content).toBe('New story');
  });

  it('does not erase an action submitted while initial history is loading', async () => {
    let finish: (messages: ChatMessage[]) => void = () => {};
    const pending = new Promise<ChatMessage[]>((resolve) => {
      finish = resolve;
    });
    fetchMock.mockReturnValue(pending);
    useStore.getState().session.activate('A', 'session-A');
    renderHook(() => useSession());
    act(() => useStore.getState().chat.appendUser('A new action'));
    await act(async () => {
      finish([]);
      await pending;
    });
    expect(useStore.getState().chat.messages[0]?.content).toBe('A new action');
  });
});
