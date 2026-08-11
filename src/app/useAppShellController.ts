import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { initBackendListener } from '../api/client';
import i18n from '../i18n';

export type SettingsTab = 'chat' | 'image' | 'video';

export function useAppShellController(input: {
  uiLanguage: string;
  chatPanelWidth: number;
  quickSave: () => Promise<unknown>;
  openSaves: () => void;
}) {
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

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key === 'F11') {
        event.preventDefault();
        const win = getCurrentWindow();
        void win.isFullscreen().then((fullscreen) => win.setFullscreen(!fullscreen));
        return;
      }
      if (event.ctrlKey && !event.shiftKey && (event.key === 's' || event.key === 'S')) {
        event.preventDefault();
        void input.quickSave();
        return;
      }
      if (!(event.ctrlKey && event.shiftKey)) return;
      if (event.key === 'M' || event.key === 'm') {
        event.preventDefault();
        setLocalModeOpen((current) => !current);
      } else if (event.key === 'I' || event.key === 'i') {
        event.preventDefault();
        setInspectorOpen((current) => !current);
      } else if (event.key === 'S' || event.key === 's') {
        event.preventDefault();
        input.openSaves();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [input.openSaves, input.quickSave]);

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
