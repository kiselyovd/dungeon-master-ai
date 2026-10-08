export interface ProviderRestorePorts {
  hydrated(): boolean;
  configured(): boolean;
  apply(): Promise<void>;
  onHydrated(callback: () => void): () => void;
  onBackendReady(callback: () => void): () => void;
  failed(): void;
}
/** Restore persisted configuration without choosing or downloading another model. */
export function restoreProviderOnStartup(ports: ProviderRestorePorts): () => void {
  let disposed = false;
  const restore = () => {
    if (disposed || !ports.hydrated() || !ports.configured()) return;
    void Promise.resolve()
      .then(() => ports.apply())
      .catch(() => {
        if (!disposed) ports.failed();
      });
  };
  const detachHydration = ports.onHydrated(restore);
  const detachBackend = ports.onBackendReady(restore);
  restore();
  return () => {
    disposed = true;
    detachHydration();
    detachBackend();
  };
}
