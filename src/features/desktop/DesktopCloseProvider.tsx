import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import { confirmDesktopExit, listenForDesktopClose } from '../../api/desktop';
import { Button } from '../../ui/Button';
import { Modal } from '../../ui/Modal';
import {
  type ClosePolicy,
  requiresCloseConfirmation,
  sameCloseScope,
  type UnconfirmedDeparture,
} from './model/closePolicy';

interface CloseContext {
  register(id: symbol, policy: () => ClosePolicy): () => void;
  request(): void;
}
const Context = createContext<CloseContext | null>(null);
const idle: ClosePolicy = { scope: 'idle', kind: 'idle', pending: false, prepare: async () => {} };
export function DesktopCloseProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation('common');
  const policies = useRef(new Map<symbol, () => ClosePolicy>());
  const [preview, setPreview] = useState<ClosePolicy | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState(false);
  const [unconfirmed, setUnconfirmed] = useState<UnconfirmedDeparture | null>(null);
  const previewRef = useRef(preview);
  previewRef.current = preview;
  const closing = useRef(false);
  const mounted = useRef(true);
  const latest = useCallback(() => {
    const current = [...policies.current.values()].map((read) => read());
    return (
      current.find((policy) => policy.kind === 'host' || policy.kind === 'guest') ??
      current.at(-1) ??
      idle
    );
  }, []);
  const complete = async (expected: ClosePolicy, preserve?: () => Promise<void>) => {
    if (closing.current) return;
    closing.current = true;
    setPreview(expected);
    setBusy(true);
    setFailed(false);
    setUnconfirmed(null);
    try {
      const current = latest();
      if (!sameCloseScope(expected, current)) throw Error('desktop_close_scope_changed');
      const prepared = preserve ? await preserve() : await current.prepare();
      // A different campaign/connection must not be closed by a stale dialog.
      if (!sameCloseScope(expected, latest())) throw Error('desktop_close_scope_changed');
      if (prepared && prepared.kind === 'unconfirmed_departure') {
        console.warn('[desktop.close]', { code: 'desktop_departure_unconfirmed' });
        if (mounted.current) {
          setUnconfirmed(prepared);
          setFailed(true);
        }
        return;
      }
      await confirmDesktopExit();
      if (mounted.current) setPreview(null);
    } catch {
      console.warn('[desktop.close]', { code: 'desktop_close_failed' });
      if (mounted.current) {
        setPreview(expected);
        setFailed(true);
      }
    } finally {
      closing.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  const completeRef = useRef(complete);
  completeRef.current = complete;
  const request = useCallback(() => {
    if (closing.current || previewRef.current) return;
    const current = latest();
    if (requiresCloseConfirmation(current)) {
      setFailed(false);
      setUnconfirmed(null);
      setPreview((previous) => previous ?? current);
    } else void completeRef.current(current);
  }, [latest]);
  const dismiss = () => {
    if (closing.current) return;
    setPreview(null);
    setUnconfirmed(null);
    setFailed(false);
  };
  const context = useMemo<CloseContext>(
    () => ({
      request,
      register(id, read) {
        policies.current.set(id, read);
        return () => {
          policies.current.delete(id);
        };
      },
    }),
    [request],
  );
  useEffect(() => {
    mounted.current = true;
    let cleanup: (() => void) | undefined;
    let canceled = false;
    void listenForDesktopClose(request)
      .then((unlisten) => {
        if (canceled) unlisten();
        else cleanup = unlisten;
      })
      .catch(() => console.warn('[desktop.close]', { code: 'desktop_close_listener_failed' }));
    return () => {
      canceled = true;
      mounted.current = false;
      cleanup?.();
    };
  }, [request]);
  return (
    <Context.Provider value={context}>
      {children}
      <Modal
        open={preview !== null}
        onClose={dismiss}
        disableClose={busy}
        title={t('desktop_close_title')}
        footer={
          <>
            <Button disabled={busy} onClick={dismiss}>
              {t('cancel')}
            </Button>
            {unconfirmed && (
              <Button
                disabled={busy}
                variant="danger"
                onClick={() => {
                  if (preview) void completeRef.current(preview, unconfirmed.preserveAndExit);
                }}
              >
                {t('desktop_close_keep_recovery')}
              </Button>
            )}
            <Button
              disabled={busy}
              variant="danger"
              onClick={() => {
                if (preview) void completeRef.current(preview);
              }}
            >
              {t(busy ? 'desktop_closing' : 'desktop_close_confirm')}
            </Button>
          </>
        }
      >
        <p>
          {t(
            preview?.kind === 'host'
              ? 'desktop_close_host'
              : preview?.kind === 'guest'
                ? 'desktop_close_guest'
                : 'desktop_close_solo',
          )}
        </p>
        {preview?.pending && <p>{t('desktop_close_pending')}</p>}
        {failed && (
          <p role="alert">
            {t(unconfirmed ? 'desktop_close_unconfirmed' : 'desktop_close_failed')}
          </p>
        )}
      </Modal>
    </Context.Provider>
  );
}
export function useDesktopCloseRequest(): () => void {
  const context = useContext(Context);
  if (!context) throw Error('desktop_close_context_missing');
  return context.request;
}
export function useDesktopClosePolicy(policy: ClosePolicy): void {
  const context = useContext(Context);
  const id = useRef(Symbol('close-policy'));
  const latest = useRef(policy);
  latest.current = policy;
  useLayoutEffect(() => context?.register(id.current, () => latest.current), [context]);
}
