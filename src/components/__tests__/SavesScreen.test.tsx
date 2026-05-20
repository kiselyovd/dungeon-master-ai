import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import '../../i18n';
import type { SaveSummary } from '../../api/saves';
import * as savesApi from '../../api/saves';
import { useStore } from '../../state/useStore';
import { SavesScreen } from '../SavesScreen';

/**
 * SavesScreen test suite (M5 P2.13).
 *
 * The hook delegates network I/O to `src/api/saves.ts`, so we mock that
 * module directly. Stubbing only `globalThis.fetch` is not enough -
 * `backendUrl()` would still call into the (un-mocked) Tauri shim and
 * never resolve, so the on-mount refresh would hang.
 */

vi.mock('../../api/saves', () => ({
  fetchSessionSaves: vi.fn(),
  createSave: vi.fn(),
  quickSaveSession: vi.fn(),
  fetchSaveById: vi.fn(),
  deleteSaveById: vi.fn(),
  fetchSessionMessages: vi.fn(),
}));

const fetchSessionSavesMock = vi.mocked(savesApi.fetchSessionSaves);
const fetchSaveByIdMock = vi.mocked(savesApi.fetchSaveById);
const deleteSaveByIdMock = vi.mocked(savesApi.deleteSaveById);
const createSaveMock = vi.mocked(savesApi.createSave);
const fetchSessionMessagesMock = vi.mocked(savesApi.fetchSessionMessages);

const FIXTURE: SaveSummary[] = [
  {
    id: 's1',
    session_id: 'sess1',
    kind: 'manual',
    title: 'Before the boss',
    summary: 'Party rests outside the lair.',
    tag: 'exploration',
    created_at: '2026-05-09T15:30:00Z',
    turn_number: 0,
  },
  {
    id: 's2',
    session_id: 'sess1',
    kind: 'auto',
    title: 'Auto checkpoint',
    summary: 'Pre-combat snapshot.',
    tag: 'combat',
    created_at: '2026-05-09T15:00:00Z',
    turn_number: 0,
  },
  {
    id: 's3',
    session_id: 'sess1',
    kind: 'manual',
    title: 'Talked to the priest',
    summary: 'Father Aldis blessed the party.',
    tag: 'dialog',
    created_at: '2026-05-09T14:00:00Z',
    turn_number: 0,
  },
];

describe('SavesScreen', () => {
  beforeEach(() => {
    useStore.setState(useStore.getInitialState());
    useStore.getState().session.setActiveSession('camp-1', 'sess1');
    fetchSessionSavesMock.mockReset();
    fetchSaveByIdMock.mockReset();
    deleteSaveByIdMock.mockReset();
    createSaveMock.mockReset();
    fetchSessionMessagesMock.mockReset();
    fetchSessionSavesMock.mockResolvedValue([...FIXTURE]);
    fetchSaveByIdMock.mockResolvedValue({
      ...FIXTURE[0],
      game_state: { schema_version: 1 },
    } as savesApi.SaveRow);
    deleteSaveByIdMock.mockResolvedValue();
    createSaveMock.mockResolvedValue({ id: 'new-1' });
    fetchSessionMessagesMock.mockResolvedValue([]);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and tab bar (no Branches tab in v1)', async () => {
    render(<SavesScreen />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/Chronicles of Adventure/i);
    expect(screen.getByRole('tab', { name: 'All' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Manual' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Auto' })).toBeInTheDocument();
    expect(screen.queryByRole('tab', { name: /Branches/i })).toBeNull();
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
  });

  it('Auto tab narrows the list to auto saves only', async () => {
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('tab', { name: 'Auto' }));
    expect(screen.queryByText('Before the boss')).toBeNull();
    expect(screen.getByText('Auto checkpoint')).toBeInTheDocument();
  });

  it('selecting a save row populates the right-page detail', async () => {
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getByText('Talked to the priest')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Talked to the priest'));
    const heading = screen.getByRole('heading', { level: 2 });
    expect(heading).toHaveTextContent('Talked to the priest');
  });

  it('clicking Load triggers fetchSaveById for the selected row', async () => {
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    // First save auto-selected on mount.
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      expect(fetchSaveByIdMock).toHaveBeenCalledWith('s1');
    });
  });

  it('clicking Delete issues deleteSaveById on the selected row', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Delete/i }));
    await waitFor(() => {
      expect(deleteSaveByIdMock).toHaveBeenCalledWith('s1');
    });
    confirmSpy.mockRestore();
  });

  it('search box filters by title and summary', async () => {
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.type(screen.getByPlaceholderText(/Search saves/i), 'priest');
    expect(screen.queryByText('Before the boss')).toBeNull();
    expect(screen.getByText('Talked to the priest')).toBeInTheDocument();
  });

  it('shows the empty-state when the session has no saves', async () => {
    fetchSessionSavesMock.mockResolvedValueOnce([]);
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getByText(/No saves yet/i)).toBeInTheDocument();
    });
  });

  it('Escape key closes the modal via the slice', async () => {
    useStore.getState().saves.open();
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      // The title appears once in the row + once in the right-page detail
      // (auto-selected on mount), so use getAllByText.
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.keyboard('{Escape}');
    await waitFor(() => {
      expect(useStore.getState().saves.isOpen).toBe(false);
    });
  });

  // C5: minimal rehydration on Load
  it('C5: fetchSessionMessages is called with the save session_id on Load', async () => {
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      expect(fetchSessionMessagesMock).toHaveBeenCalledWith('sess1', { limit: 20 });
    });
  });

  it('C5: chat store is populated with the fetched messages after Load', async () => {
    const wireMessages: savesApi.SessionMessageWire[] = [
      { role: 'user', content: 'Hello dungeon master', parts: [] },
      { role: 'assistant', content: 'Welcome, adventurer', parts: [] },
    ];
    fetchSessionMessagesMock.mockResolvedValue(wireMessages);
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      const messages = useStore.getState().chat.messages;
      expect(messages).toHaveLength(2);
      const [msg0, msg1] = messages;
      expect(msg0?.role).toBe('user');
      expect(msg0?.content).toBe('Hello dungeon master');
      expect(msg1?.role).toBe('assistant');
      expect(msg1?.content).toBe('Welcome, adventurer');
    });
  });

  it('C5: the modal closes on successful Load', async () => {
    useStore.getState().saves.open();
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      expect(useStore.getState().saves.isOpen).toBe(false);
    });
  });

  it('C5: an inline error is shown and modal stays open when Load fails', async () => {
    useStore.getState().saves.open();
    fetchSessionMessagesMock.mockRejectedValue(new Error('network error'));
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      // Modal stays open
      expect(useStore.getState().saves.isOpen).toBe(true);
      // Inline error message uses the load_error i18n key with the error message interpolated
      const alert = screen.getByRole('alert');
      expect(alert).toBeInTheDocument();
      expect(alert).toHaveTextContent('Save could not be loaded: network error');
    });
  });

  it('C5: only the last 20 messages populate the chat when the backend returns more than 20', async () => {
    // Build 25 wire messages; the client should keep only the last 20.
    const wireMessages: savesApi.SessionMessageWire[] = Array.from({ length: 25 }, (_, i) => ({
      role: (i % 2 === 0 ? 'user' : 'assistant') as savesApi.SessionMessageWire['role'],
      content: `message-${i}`,
      parts: [],
    }));
    fetchSessionMessagesMock.mockResolvedValue(wireMessages);
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      const messages = useStore.getState().chat.messages;
      // Only the last 20 of the 25 wire messages should appear.
      expect(messages).toHaveLength(20);
      // The last message in the store should be message-24 (the final wire message).
      expect(messages[messages.length - 1]?.content).toBe('message-24');
      // The first message in the store should be message-5 (wire index 25-20=5).
      expect(messages[0]?.content).toBe('message-5');
    });
  });

  it('C5: tool_call and tool_result messages are excluded from the chat on Load', async () => {
    // A realistic agent session: system prompt, then interleaved user/assistant/tool messages.
    const wireMessages: savesApi.SessionMessageWire[] = [
      { role: 'system', content: 'You are a dungeon master.', parts: [] },
      { role: 'user', content: 'I search the room.', parts: [] },
      {
        role: 'assistant_with_tool_calls',
        content: '',
        tool_calls: [{ id: 'tc1', name: 'search_room', arguments: '{}' }],
        parts: [],
      },
      { role: 'tool_result', content: 'You find a sword.', parts: [] },
      { role: 'assistant', content: 'You find a gleaming sword!', parts: [] },
    ];
    fetchSessionMessagesMock.mockResolvedValue(wireMessages);
    const user = userEvent.setup();
    render(<SavesScreen />);
    await waitFor(() => {
      expect(screen.getAllByText('Before the boss').length).toBeGreaterThan(0);
    });
    await user.click(screen.getByRole('button', { name: /Load/i }));
    await waitFor(() => {
      const messages = useStore.getState().chat.messages;
      // Only system, user, and assistant roles should be rehydrated (3 total).
      expect(messages).toHaveLength(3);
      expect(messages[0]?.role).toBe('system');
      expect(messages[1]?.role).toBe('user');
      expect(messages[2]?.role).toBe('assistant');
      // The assistant_with_tool_calls and tool_result entries must be absent.
      expect(messages.find((m) => m.content === 'You find a sword.')).toBeUndefined();
    });
  });
});
