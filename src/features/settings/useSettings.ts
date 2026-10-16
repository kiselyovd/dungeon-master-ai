import { useRef, useState } from 'react';
import { postSettingsV2 } from '../../api/settings';
import { useStore } from '../../state/useStore';
import type { SettingsSubmission } from './model/draft';

/** One acknowledged replacement, including all preferences. Drafts stay in the form. */
export function useSettings() {
  const saving = useRef(false);
  const [busy, setBusy] = useState(false);
  const save = async (draft: SettingsSubmission): Promise<boolean> => {
    if (saving.current) return false;
    saving.current = true;
    setBusy(true);
    const state = useStore.getState();
    const replacement = {
      ...draft.preferences,
      activeProvider: draft.provider.kind,
      providers: { ...state.settings.providers, [draft.provider.kind]: draft.provider },
      uiLanguage: draft.uiLanguage,
      narrationLanguage: draft.narrationLanguage,
      systemPrompt: draft.systemPrompt,
      temperature: draft.temperature,
      replicateApiKey: draft.replicateApiKey || null,
    };
    try {
      await postSettingsV2({ ...state.settings, ...replacement }, state.localMode);
      // Keep independent workspace changes (panel width/catalogs) made during the ACK.
      useStore.setState((current) => ({ settings: { ...current.settings, ...replacement } }));
      if (import.meta.env.VITE_RECOVERY_TRACE === '1') console.debug('[settings] acknowledged');
      return true;
    } finally {
      saving.current = false;
      setBusy(false);
    }
  };
  return { save, busy };
}
