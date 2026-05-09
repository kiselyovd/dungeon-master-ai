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
}));

const fetchSessionSavesMock = vi.mocked(savesApi.fetchSessionSaves);
const fetchSaveByIdMock = vi.mocked(savesApi.fetchSaveById);
const deleteSaveByIdMock = vi.mocked(savesApi.deleteSaveById);
const createSaveMock = vi.mocked(savesApi.createSave);

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
    fetchSessionSavesMock.mockResolvedValue([...FIXTURE]);
    fetchSaveByIdMock.mockResolvedValue({
      ...FIXTURE[0],
      game_state: { schema_version: 1 },
    } as savesApi.SaveRow);
    deleteSaveByIdMock.mockResolvedValue();
    createSaveMock.mockResolvedValue({ id: 'new-1' });
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
});
