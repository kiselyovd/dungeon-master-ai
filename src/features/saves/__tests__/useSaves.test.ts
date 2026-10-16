import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../../api/saves';
import { useStore } from '../../../state/useStore';
import { useSaves } from '../useSaves';
import '../../../i18n';

vi.mock('../../../api/saves', async (original) => ({
  ...(await original<typeof import('../../../api/saves')>()),
  fetchSessionSaves: vi.fn(),
  quickSaveSession: vi.fn(),
  createSave: vi.fn(),
  updateSaveById: vi.fn(),
  deleteSaveById: vi.fn(),
  restoreSave: vi.fn(),
}));

function activate(id: string) {
  useStore.getState().session.activate(id, `session-${id}`);
  useStore.getState().campaigns.setActive({
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
  });
}

beforeEach(() => {
  useStore.setState(useStore.getInitialState());
  vi.resetAllMocks();
  activate('A');
});

describe('save operation ownership', () => {
  it('retries a committed restore with a lost response using the exact original request', async () => {
    vi.mocked(api.restoreSave)
      .mockRejectedValueOnce(new TypeError('response lost after commit'))
      .mockResolvedValueOnce({
        id: 'old-save',
        campaign_id: 'A',
        session_id: 'session-A',
        revision: 1,
        current_revision: 2,
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        projection: {
          character: { name: 'Recovered' },
          messages: [],
          combat: null,
          scene: null,
          journal: [],
          npcs: [],
        },
      });
    const { result, unmount } = renderHook(() => useSaves());
    await act(async () => {
      await result.current.rehydrateFromSave('old-save');
    });
    unmount();
    const reopened = renderHook(() => useSaves());
    await act(async () => {
      await reopened.result.current.rehydrateFromSave('old-save');
    });
    expect(vi.mocked(api.restoreSave).mock.calls[1]).toEqual(
      vi.mocked(api.restoreSave).mock.calls[0],
    );
    expect(useStore.getState().campaigns.active?.revision).toBe(2);
  });
  it('serializes restore with checkpoint creation and rejects a pre-restore save intent', async () => {
    let finish: (value: api.RestoreSaveResponse) => void = () => {};
    const pending = new Promise<api.RestoreSaveResponse>((resolve) => {
      finish = resolve;
    });
    vi.mocked(api.restoreSave).mockReturnValue(pending);
    vi.mocked(api.quickSaveSession).mockResolvedValue({ id: 'unexpected', revision: 2 });
    vi.mocked(api.fetchSessionSaves).mockResolvedValue([]);
    const { result } = renderHook(() => useSaves());
    let restoration: Promise<unknown> = Promise.resolve();
    act(() => {
      restoration = result.current.rehydrateFromSave('old-save');
    });
    await waitFor(() => expect(api.restoreSave).toHaveBeenCalled());
    let save: Promise<unknown> = Promise.resolve();
    await act(async () => {
      save = result.current.quickSave();
      await Promise.resolve();
    });
    expect(api.quickSaveSession).not.toHaveBeenCalled();
    await act(async () => {
      finish({
        id: 'old-save',
        campaign_id: 'A',
        session_id: 'session-A',
        revision: 1,
        schema_version: 1,
        ruleset_id: 'srd-5.2.1',
        projection: {
          character: {},
          messages: [],
          combat: null,
          scene: null,
          journal: [],
          npcs: [],
        },
      });
      await restoration;
      await save;
    });
    expect(api.quickSaveSession).not.toHaveBeenCalled();
  });
  it.each([
    'refresh',
    'quickSave',
    'manualSave',
    'overwriteSave',
    'deleteSave',
  ] as const)('ignores late %s results from the previous campaign', async (method) => {
    let finish: (value?: unknown) => void = () => {};
    const pending = new Promise<unknown>((resolve) => {
      finish = resolve;
    });
    const oldList = [
      {
        id: 'save-A',
        session_id: 'session-A',
        kind: 'manual',
        title: 'A',
        summary: '',
        tag: 'exploration',
        created_at: '',
        turn_number: 0,
      },
    ] as api.SaveSummary[];
    vi.mocked(api.fetchSessionSaves).mockResolvedValue(oldList);
    if (method === 'refresh')
      vi.mocked(api.fetchSessionSaves).mockReturnValue(pending as Promise<api.SaveSummary[]>);
    if (method === 'quickSave')
      vi.mocked(api.quickSaveSession).mockReturnValue(pending as Promise<api.SaveCreated>);
    if (method === 'manualSave')
      vi.mocked(api.createSave).mockReturnValue(pending as Promise<api.SaveCreated>);
    if (method === 'overwriteSave')
      vi.mocked(api.updateSaveById).mockReturnValue(pending as Promise<void>);
    if (method === 'deleteSave')
      vi.mocked(api.deleteSaveById).mockReturnValue(pending as Promise<void>);
    const { result } = renderHook(() => useSaves());
    let operation: Promise<unknown> = Promise.resolve();
    act(() => {
      const body = { kind: 'manual', title: 'A', summary: '', tag: 'exploration' } as const;
      operation =
        method === 'manualSave'
          ? result.current.manualSave(body)
          : method === 'overwriteSave'
            ? result.current.overwriteSave('save-A', body)
            : method === 'deleteSave'
              ? result.current.deleteSave('save-A')
              : result.current[method]();
    });
    await waitFor(() =>
      expect(
        method === 'refresh'
          ? api.fetchSessionSaves
          : method === 'quickSave'
            ? api.quickSaveSession
            : method === 'manualSave'
              ? api.createSave
              : method === 'overwriteSave'
                ? api.updateSaveById
                : api.deleteSaveById,
      ).toHaveBeenCalled(),
    );
    act(() => {
      activate('B');
      useStore.getState().saves.setList([]);
      useStore.getState().saves.setLastSaveError('B error');
    });
    await act(async () => {
      finish(method === 'refresh' ? oldList : { id: 'save-A', revision: 2 });
      await operation;
    });
    expect(useStore.getState().saves.list).toEqual([]);
    expect(useStore.getState().saves.lastSaveError).toBe('B error');
    expect(useStore.getState().saves.lastQuickSaveAt).toBeNull();
  });

  it('retries an unacknowledged quick save with the exact original causation and revision', async () => {
    vi.mocked(api.quickSaveSession)
      .mockRejectedValueOnce(new Error('response lost'))
      .mockResolvedValueOnce({ id: 'committed', revision: 2 });
    vi.mocked(api.fetchSessionSaves).mockResolvedValue([]);
    const { result } = renderHook(() => useSaves());
    await act(async () => {
      await result.current.quickSave();
    });
    await act(async () => {
      await result.current.quickSave();
    });
    expect(vi.mocked(api.quickSaveSession).mock.calls[1]?.[1]).toEqual(
      vi.mocked(api.quickSaveSession).mock.calls[0]?.[1],
    );
    expect(useStore.getState().campaigns.active?.revision).toBe(2);
  });
});
