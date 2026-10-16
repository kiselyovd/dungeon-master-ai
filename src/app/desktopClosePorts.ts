import { flushClosePreparations } from '../state/closePreparations';
import { flushConversationDrafts } from '../state/conversationDrafts';
import { persistStorage } from '../state/persistStorage';
import { useStore } from '../state/useStore';

/** Disk barriers use the existing allowlisted settings/Stronghold adapters. */
export async function flushAppCloseState(): Promise<void> {
  await flushClosePreparations();
  await flushConversationDrafts();
  const snapshot = useStore.persist.getOptions().partialize?.(useStore.getState()) ?? {};
  await persistStorage.setItem('dmai-settings', { state: snapshot, version: 0 });
}
