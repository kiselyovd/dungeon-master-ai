import { useEffect, useState } from 'react';
import { initBackendListener } from '../api/client';
import i18n from '../i18n';

export type SettingsTab = 'chat' | 'local-llm' | 'image' | 'video' | 'behavior';

export function useAppShellController(input: { uiLanguage: string; chatPanelWidth: number }) {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsInitialTab, setSettingsInitialTab] = useState<SettingsTab>('chat');
  const [localModeOpen, setLocalModeOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [characterSheetOpen, setCharacterSheetOpen] = useState(false);
  const [wizardReopen, setWizardReopen] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void initBackendListener().then((cleanup) => {
      if (cancelled) cleanup();
      else unlisten = cleanup;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (i18n.language !== input.uiLanguage) void i18n.changeLanguage(input.uiLanguage);
  }, [input.uiLanguage]);

  useEffect(() => {
    document.documentElement.style.setProperty('--chat-width', `${input.chatPanelWidth}px`);
  }, [input.chatPanelWidth]);

  return {
    settingsOpen,
    setSettingsOpen,
    settingsInitialTab,
    setSettingsInitialTab,
    localModeOpen,
    setLocalModeOpen,
    inspectorOpen,
    setInspectorOpen,
    characterSheetOpen,
    setCharacterSheetOpen,
    wizardReopen,
    setWizardReopen,
  };
}
