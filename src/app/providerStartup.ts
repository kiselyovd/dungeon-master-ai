import { subscribeBackendReady } from '../api/client';
import { ChatError } from '../api/errors';
import { ensureSettingsV2, invalidateBackendSettings } from '../api/settings';
import { restoreProviderOnStartup } from '../features/provider-setup/restoreProvider';
import { useStore } from '../state/useStore';

/** Composition adapter: configured provider values come only from hydrated storage. */
export async function prepareBackendProvider(): Promise<void> {
  const state = useStore.getState();
  if (!useStore.persist.hasHydrated() || !state.settings.providers[state.settings.activeProvider])
    throw new ChatError('provider_error', 'provider_not_configured');
  return ensureSettingsV2(state.settings, state.localMode);
}
export function startProviderStartup(): () => void {
  return restoreProviderOnStartup({
    hydrated: () => useStore.persist.hasHydrated(),
    configured: () => {
      const state = useStore.getState();
      return state.settings.providers[state.settings.activeProvider] !== null;
    },
    apply: prepareBackendProvider,
    onHydrated: (callback) => useStore.persist.onFinishHydration(callback),
    onBackendReady: (callback) =>
      subscribeBackendReady(() => {
        invalidateBackendSettings();
        callback();
      }),
    failed: () =>
      useStore
        .getState()
        .chat.setError({ code: 'provider_error', message: 'provider_restore_failed' }),
  });
}
