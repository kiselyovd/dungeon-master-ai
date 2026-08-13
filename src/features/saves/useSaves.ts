import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  createSave as apiCreateSave,
  type CreateSaveRequest,
  deleteSaveById,
  fetchSaveById,
  fetchSessionSaves,
  quickSaveSession,
  restoreSave,
  type SaveSummary,
  updateSaveById,
} from '../../api/saves';
import { useStore } from '../../state/useStore';
import { buildRestoredSession, RestoreValidationError } from './model/buildRestoredSession';

export type RehydrateResult = { ok: true } | { ok: false; error: string };

export interface UseSavesResult {
  saves: SaveSummary[];
  selectedSaveId: string | null;
  isOpen: boolean;
  lastQuickSaveAt: string | null;
  refresh: () => Promise<void>;
  quickSave: () => Promise<{ id: string } | null>;
  manualSave: (body: CreateSaveRequest) => Promise<{ id: string } | null>;
  overwriteSave: (saveId: string, body: CreateSaveRequest) => Promise<boolean>;
  rehydrateFromSave: (saveId: string) => Promise<RehydrateResult>;
  deleteSave: (saveId: string) => Promise<void>;
  open: () => void;
  close: () => void;
  selectSave: (id: string | null) => void;
}

let nextOperation = 0;

export function useSaves(): UseSavesResult {
  const { t } = useTranslation('saves');
  const saves = useStore((state) => state.saves.list);
  const selectedSaveId = useStore((state) => state.saves.selectedSaveId);
  const isOpen = useStore((state) => state.saves.isOpen);
  const lastQuickSaveAt = useStore((state) => state.saves.lastQuickSaveAt);
  const setList = useStore((state) => state.saves.setList);
  const selectSave = useStore((state) => state.saves.selectSave);
  const open = useStore((state) => state.saves.open);
  const close = useStore((state) => state.saves.close);
  const setLastQuickSaveAt = useStore((state) => state.saves.setLastQuickSaveAt);
  const setLastSaveError = useStore((state) => state.saves.setLastSaveError);
  const currentSession = useStore((state) => state.session.current);
  const applyRestoredSession = useStore((state) => state.restoration.apply);

  const refresh = useCallback(async () => {
    const active = currentSession();
    if (active === null) {
      setLastSaveError(t('campaign_required'));
      return;
    }
    const { sessionId } = active;
    setList(await fetchSessionSaves(sessionId));
  }, [currentSession, setLastSaveError, setList, t]);

  const quickSave = useCallback(async () => {
    const active = currentSession();
    if (active === null) {
      setLastSaveError(t('campaign_required'));
      return null;
    }
    const { sessionId } = active;
    try {
      const result = await quickSaveSession(sessionId);
      setLastQuickSaveAt(new Date().toISOString());
      try {
        setList(await fetchSessionSaves(sessionId));
      } catch {
        // The save succeeded; the next refresh will reconcile the list.
      }
      setLastSaveError(null);
      return result;
    } catch (error) {
      console.error('[save.operation]', { code: 'quick_save_failed', sessionId });
      setLastSaveError(errorMessage(error));
      return null;
    }
  }, [currentSession, setLastQuickSaveAt, setList, setLastSaveError, t]);

  const manualSave = useCallback(
    async (body: CreateSaveRequest) => {
      const active = currentSession();
      if (active === null) {
        setLastSaveError(t('campaign_required'));
        return null;
      }
      const { sessionId } = active;
      try {
        const result = await apiCreateSave(sessionId, body);
        setList(await fetchSessionSaves(sessionId));
        setLastSaveError(null);
        return result;
      } catch (error) {
        console.error('[save.operation]', { code: 'manual_save_failed', sessionId });
        setLastSaveError(errorMessage(error));
        return null;
      }
    },
    [currentSession, setList, setLastSaveError, t],
  );

  const overwriteSave = useCallback(
    async (saveId: string, body: CreateSaveRequest) => {
      const active = currentSession();
      if (active === null) {
        setLastSaveError(t('campaign_required'));
        return false;
      }
      const { sessionId } = active;
      try {
        await updateSaveById(saveId, body);
        setList(await fetchSessionSaves(sessionId));
        setLastSaveError(null);
        return true;
      } catch (error) {
        console.error('[save.operation]', { code: 'overwrite_save_failed', saveId, sessionId });
        setLastSaveError(errorMessage(error));
        return false;
      }
    },
    [currentSession, setList, setLastSaveError, t],
  );

  const rehydrateFromSave = useCallback(
    async (saveId: string): Promise<RehydrateResult> => {
      const operationId = `restore-${++nextOperation}`;
      const startedAt = performance.now();
      try {
        const active = currentSession();
        if (active === null) {
          return { ok: false, error: t('campaign_required') };
        }
        // The read-only row lookup resolves the target session. The mutating
        // restore response then includes both game state and messages, so no
        // fallible network call occurs after backend mutation.
        const row = await fetchSaveById(saveId);
        const { campaignId } = active;
        const restored = await restoreSave(saveId, row.session_id);
        const projection = buildRestoredSession({ saveId, campaignId, row, restored });
        applyRestoredSession(projection);
        console.debug('[save.restore]', {
          operationId,
          saveId,
          sessionId: projection.sessionId,
          messageCount: projection.messages.length,
          combatActive: projection.combat?.active ?? false,
          durationMs: Math.round(performance.now() - startedAt),
          transition: 'committed',
        });
        return { ok: true };
      } catch (error) {
        const code = error instanceof RestoreValidationError ? error.code : 'restore_failed';
        console.warn('[save.restore]', {
          operationId,
          saveId,
          code,
          durationMs: Math.round(performance.now() - startedAt),
          transition: 'rejected',
        });
        return { ok: false, error: errorMessage(error) };
      }
    },
    [applyRestoredSession, currentSession, t],
  );

  const deleteSave = useCallback(
    async (saveId: string) => {
      try {
        await deleteSaveById(saveId);
        await refresh();
        setLastSaveError(null);
      } catch (error) {
        console.error('[save.operation]', { code: 'delete_save_failed', saveId });
        setLastSaveError(errorMessage(error));
      }
    },
    [refresh, setLastSaveError],
  );

  return useMemo(
    () => ({
      saves,
      selectedSaveId,
      isOpen,
      lastQuickSaveAt,
      refresh,
      quickSave,
      manualSave,
      overwriteSave,
      rehydrateFromSave,
      deleteSave,
      open,
      close,
      selectSave,
    }),
    [
      saves,
      selectedSaveId,
      isOpen,
      lastQuickSaveAt,
      refresh,
      quickSave,
      manualSave,
      overwriteSave,
      rehydrateFromSave,
      deleteSave,
      open,
      close,
      selectSave,
    ],
  );
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : 'Load failed';
}
