import { renderHook, waitFor } from '@testing-library/react';
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

  it('mints a fresh (campaignId, sessionId) on first mount', async () => {
    fetchMock.mockResolvedValue([]);

    expect(useStore.getState().session.activeCampaignId).toBeNull();
    expect(useStore.getState().session.activeSessionId).toBeNull();

    renderHook(() => useSession());

    await waitFor(() => {
      expect(useStore.getState().session.activeCampaignId).not.toBeNull();
      expect(useStore.getState().session.activeSessionId).not.toBeNull();
    });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const sessionId = useStore.getState().session.activeSessionId;
    expect(fetchMock).toHaveBeenCalledWith(sessionId);
  });

  it('seeds the chat slice with persisted history when the backend returns messages', async () => {
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
    fetchMock.mockRejectedValue(new Error('backend down'));

    renderHook(() => useSession());

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalled();
    });
    expect(useStore.getState().chat.messages).toEqual([]);
  });

  it('does not overwrite the chat slice when the backend returns no messages', async () => {
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
    expect(useStore.getState().chat.messages).toEqual([
      { id: 'pre-existing', role: 'user', content: 'staged' },
    ]);
  });
});
