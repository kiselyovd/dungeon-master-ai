import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import {
  type InvitationEvent,
  type InvitationPreview,
  listenForInvitations,
  takeInvitation,
} from '../../api/invitations';
import { Button } from '../../ui/Button';
import { Modal } from '../../ui/Modal';

interface Target {
  scope: string;
  priority: number;
  blocked: boolean;
  prepare(preview: InvitationPreview): Promise<() => void>;
}
interface Registry {
  register(id: symbol, read: () => Target): () => void;
}
const Context = createContext<Registry | null>(null);
export function DesktopInvitationsProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation('rooms');
  const targets = useRef(new Map<symbol, () => Target>());
  const [, refresh] = useReducer((value) => value + 1, 0);
  const [queue, setQueue] = useState<InvitationEvent[]>([]);
  const queued = useRef(queue);
  queued.current = queue;
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const draining = useRef(false);
  const mounted = useRef(true);
  const registry = useMemo<Registry>(
    () => ({
      register(id, read) {
        targets.current.set(id, read);
        refresh();
        return () => {
          targets.current.delete(id);
          refresh();
        };
      },
    }),
    [],
  );
  const drain = useCallback(async () => {
    if (draining.current) return;
    draining.current = true;
    try {
      while (mounted.current && queued.current.length < 4) {
        const event = await takeInvitation();
        if (!event || !mounted.current) break;
        if (
          queued.current.some(
            (item) =>
              item.preview.address === event.preview.address &&
              item.preview.room_id === event.preview.room_id &&
              item.preview.invitation === event.preview.invitation,
          )
        )
          continue;
        queued.current = [...queued.current, event];
        setQueue(queued.current);
      }
    } catch {
      if (mounted.current) setFailed(true);
    } finally {
      draining.current = false;
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    let cleanup: (() => void) | undefined;
    void listenForInvitations(() => {
      void drain();
    })
      .then((unlisten) => {
        if (mounted.current) {
          cleanup = unlisten;
          void drain();
        } else unlisten();
      })
      .catch(() => {
        if (mounted.current) setFailed(true);
      });
    return () => {
      mounted.current = false;
      cleanup?.();
    };
  }, [drain]);
  useEffect(() => {
    if (queue.length < 4) void drain();
  }, [queue.length, drain]);
  const current = queue[0];
  const latestTarget = () =>
    [...targets.current.values()].map((read) => read()).sort((a, b) => b.priority - a.priority)[0];
  const target = latestTarget();
  const dismiss = () => {
    if (busy) return;
    queued.current = queued.current.slice(1);
    setQueue(queued.current);
    setFailed(false);
  };
  const accept = async () => {
    const destination = latestTarget();
    if (!current || busy || !destination || destination.blocked) return;
    setBusy(true);
    setFailed(false);
    try {
      const commit = await destination.prepare(current.preview);
      const latest = latestTarget();
      if (!latest || latest.blocked || latest.scope !== destination.scope)
        throw Error('native_invitation_scope_changed');
      commit();
      if (mounted.current) {
        queued.current = queued.current.filter((item) => item.event_id !== current.event_id);
        setQueue(queued.current);
      }
    } catch {
      if (mounted.current) setFailed(true);
    } finally {
      if (mounted.current) setBusy(false);
    }
  };
  return (
    <Context.Provider value={registry}>
      {children}
      <Modal
        open={Boolean(current)}
        title={t('link_preview')}
        onClose={dismiss}
        footer={
          <>
            <Button disabled={busy} autoFocus onClick={dismiss}>
              {t('link_cancel')}
            </Button>
            <Button
              disabled={busy || !target || target.blocked}
              variant="primary"
              onClick={() => void accept()}
            >
              {t('link_open')}
            </Button>
          </>
        }
      >
        <p>{t('link_preview_hint')}</p>
        <p>
          {t('address')}: {current?.preview.address}
        </p>
        <p>
          {t('room_id')}: {current?.preview.room_id}
        </p>
        {target?.blocked && <p role="status">{t('link_busy')}</p>}
        {failed && <p role="alert">{t('link_error')}</p>}
      </Modal>
    </Context.Provider>
  );
}
export function useInvitationTarget(target: Target): void {
  const context = useContext(Context);
  const id = useRef(Symbol('invitation-target'));
  const latest = useRef(target);
  latest.current = target;
  // biome-ignore lint/correctness/useExhaustiveDependencies: scope/eligibility changes must refresh the registry's visible controls.
  useLayoutEffect(
    () => context?.register(id.current, () => latest.current),
    [context, target.scope, target.blocked],
  );
}
