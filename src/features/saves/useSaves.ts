import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { ChatError } from '../../api/errors';
import {
  createSave as apiCreateSave,
  type CreateSaveRequest,
  deleteSaveById,
  fetchSessionSaves,
  quickSaveSession,
  restoreSave,
  type SaveSummary,
  updateSaveById,
} from '../../api/saves';
import { PC_DATA_KEYS, type PcData } from '../../state/pc';
import type { PendingRestore } from '../../state/session';
import { useStore } from '../../state/useStore';
import { buildRestoredSession, RestoreValidationError } from './model/buildRestoredSession';
import { SaveRequests } from './model/saveRequests';

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
  const restoreRetry = useMemo(
    () => ({
      get current(): PendingRestore | null {
        return useStore.getState().session.pendingRestore;
      },
      set current(value: PendingRestore | null) {
        useStore.getState().session.setRestoreRetry(value);
      },
    }),
    [],
  );
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
    const scope = saveScope();
    if (!scope) {
      setLastSaveError(t('campaign_required'));
      return;
    }
    try {
      const list = await fetchSessionSaves(scope.sessionId);
      if (!ownsScope(scope)) return;
      setList(list);
      reconcileListRevision(scope.campaignId, list);
    } catch (error) {
      if (ownsScope(scope)) setLastSaveError(errorMessage(error));
    }
  }, [setLastSaveError, setList, t]);

  const writeSave = useCallback(
    async (kind: 'quick' | 'manual', body?: CreateSaveRequest) => {
      if (useStore.getState().session.mutationId) {
        setLastSaveError(t('campaign_action_pending'));
        return null;
      }
      const scope = saveScope();
      if (!scope) {
        setLastSaveError(t('campaign_required'));
        return null;
      }
      const character = structuredClone(
        Object.fromEntries(
          Object.entries(useStore.getState().pc).filter(([key]) =>
            PC_DATA_KEYS.has(key as keyof PcData),
          ),
        ),
      );
      try {
        const result = await saveRequests.write({
          kind,
          campaignId: scope.campaignId,
          sessionId: scope.sessionId,
          character,
          ...(body ? { body } : {}),
          current: () => ownsScope(scope),
          revision: () => useStore.getState().campaigns.active?.revision ?? 0,
          reconcile: (receipt) => {
            if (receipt.revision !== undefined)
              useStore.getState().campaigns.updateRevision(scope.campaignId, receipt.revision);
          },
        });
        if (!result || !ownsScope(scope)) return null;
        if (kind === 'quick') setLastQuickSaveAt(new Date().toISOString());
        try {
          const list = await fetchSessionSaves(scope.sessionId);
          if (!ownsScope(scope)) return null;
          setList(list);
          reconcileListRevision(scope.campaignId, list);
        } catch {
          // Creation was acknowledged; list hydration may be retried independently.
        }
        if (!ownsScope(scope)) return null;
        setLastSaveError(null);
        return result;
      } catch (error) {
        if (ownsScope(scope)) {
          console.error('[save.operation]', {
            code: `${kind}_save_failed`,
            sessionId: scope.sessionId,
          });
          setLastSaveError(errorMessage(error));
        }
        return null;
      }
    },
    [setLastQuickSaveAt, setLastSaveError, setList, t],
  );

  const quickSave = useCallback(() => writeSave('quick'), [writeSave]);
  const manualSave = useCallback(
    (body: CreateSaveRequest) => writeSave('manual', body),
    [writeSave],
  );

  const overwriteSave = useCallback(
    async (saveId: string, body: CreateSaveRequest) => {
      const scope = saveScope();
      if (!scope) {
        setLastSaveError(t('campaign_required'));
        return false;
      }
      try {
        await updateSaveById(saveId, body);
        if (!ownsScope(scope)) return false;
        const list = await fetchSessionSaves(scope.sessionId);
        if (!ownsScope(scope)) return false;
        setList(list);
        setLastSaveError(null);
        return true;
      } catch (error) {
        if (ownsScope(scope)) setLastSaveError(errorMessage(error));
        return false;
      }
    },
    [setList, setLastSaveError, t],
  );

  const rehydrateFromSave = useCallback(
    async (saveId: string): Promise<RehydrateResult> => {
      const scope = saveScope();
      if (!scope) return { ok: false, error: t('campaign_required') };
      return saveRequests.serialize(`${scope.campaignId}/${scope.sessionId}`, async () => {
        if (!ownsScope(scope)) return { ok: false, error: t('restore_failed') };
        const operationId = `restore-${++nextOperation}`;
        const startedAt = performance.now();
        let mutationId: string | null = null;
        try {
          const active = currentSession();
          const activeCampaign = useStore.getState().campaigns.active;
          if (active === null) {
            return { ok: false, error: t('campaign_required') };
          }
          if (activeCampaign === null) {
            return { ok: false, error: t('campaign_required') };
          }
          const { campaignId, sessionId } = active;
          const generation = useStore.getState().session.transitionGeneration;
          const key = `${campaignId}/${sessionId}`;
          const previous = restoreRetry.current?.scope === key ? restoreRetry.current : null;
          if (previous && previous.saveId !== saveId)
            return { ok: false, error: t('restore_retry_previous') };
          if (useStore.getState().chat.isStreaming)
            return { ok: false, error: t('campaign_action_pending') };
          const request = previous?.request ?? {
            campaignId,
            sessionId,
            expectedRevision: activeCampaign.revision,
            causationId: `restore-${crypto.randomUUID()}`,
          };
          mutationId = request.causationId;
          if (!useStore.getState().session.beginMutation(mutationId))
            return { ok: false, error: t('campaign_action_pending') };
          restoreRetry.current = { scope: key, saveId, request };
          const restored = await restoreSave(saveId, request);
          const current = useStore.getState().session;
          if (
            current.transitionGeneration !== generation ||
            current.activeCampaignId !== campaignId ||
            current.activeSessionId !== sessionId
          ) {
            if (restoreRetry.current?.request === request) restoreRetry.current = null;
            return { ok: false, error: t('restore_failed') };
          }
          const projection = buildRestoredSession({
            saveId,
            campaignId,
            sessionId,
            expectedRevision: request.expectedRevision,
            restored,
          });
          applyRestoredSession(projection);
          restoreRetry.current = null;
          console.debug('[save.restore]', {
            operationId,
            saveId,
            sessionId: projection.sessionId,
            messageCount: projection.messages.length,
            combatActive: projection.combat?.snapshot.active ?? false,
            durationMs: Math.round(performance.now() - startedAt),
            transition: 'committed',
          });
          return { ok: true };
        } catch (error) {
          if (error instanceof ChatError && error.code === 'http_error')
            restoreRetry.current = null;
          const code = error instanceof RestoreValidationError ? error.code : 'restore_failed';
          console.warn('[save.restore]', {
            operationId,
            saveId,
            code,
            durationMs: Math.round(performance.now() - startedAt),
            transition: 'rejected',
          });
          return { ok: false, error: errorMessage(error) };
        } finally {
          if (
            mutationId &&
            (restoreRetry.current?.request.causationId !== mutationId || !ownsScope(scope))
          ) {
            useStore.getState().session.endMutation(mutationId);
          }
        }
      });
    },
    [applyRestoredSession, currentSession, t, restoreRetry],
  );

  const deleteSave = useCallback(
    async (saveId: string) => {
      const scope = saveScope();
      if (!scope) {
        setLastSaveError(t('campaign_required'));
        return;
      }
      try {
        await deleteSaveById(saveId);
        if (!ownsScope(scope)) return;
        const list = await fetchSessionSaves(scope.sessionId);
        if (!ownsScope(scope)) return;
        setList(list);
        setLastSaveError(null);
      } catch (error) {
        if (ownsScope(scope)) setLastSaveError(errorMessage(error));
      }
    },
    [setList, setLastSaveError, t],
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

interface SaveScope {
  campaignId: string;
  sessionId: string;
  generation: number;
}
function saveScope(): SaveScope | null {
  const state = useStore.getState();
  const current = state.session.current();
  if (
    !current ||
    state.campaigns.active?.id !== current.campaignId ||
    state.campaigns.active.last_session_id !== current.sessionId
  )
    return null;
  return { ...current, generation: state.session.transitionGeneration };
}
function ownsScope(scope: SaveScope): boolean {
  const state = useStore.getState();
  return (
    state.session.activeCampaignId === scope.campaignId &&
    state.session.activeSessionId === scope.sessionId &&
    state.session.transitionGeneration === scope.generation
  );
}
function reconcileListRevision(campaignId: string, list: SaveSummary[]): void {
  const revisions = list
    .filter((save) => save.campaign_id === campaignId)
    .map((save) => save.revision ?? 0);
  if (revisions.length)
    useStore.getState().campaigns.updateRevision(campaignId, Math.max(...revisions));
}
const saveRequests = new SaveRequests((request) =>
  request.kind === 'quick'
    ? quickSaveSession(request.sessionId, request.context)
    : apiCreateSave(request.sessionId, request.body as CreateSaveRequest, request.context),
);
