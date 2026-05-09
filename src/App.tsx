import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { initBackendListener } from './api/client';
import { ActionBar } from './components/ActionBar';
import { ChatPanel } from './components/ChatPanel';
import { ChatResizer } from './components/ChatResizer';
import { InitiativeTracker } from './components/InitiativeTracker';
import { JournalViewer } from './components/JournalViewer';
import { LocalModeModal } from './components/LocalModeModal';
import { NpcMemoryGrid } from './components/NpcMemoryGrid';
import { Onboarding } from './components/Onboarding';
import { SavesScreen } from './components/SavesScreen';
import { ScenePill } from './components/ScenePill';
import { SettingsModal } from './components/SettingsModal';
import { StatusBar } from './components/StatusBar';
import { ToolInspectorDrawer } from './components/ToolInspectorDrawer';
import { UpdateAvailableModal } from './components/UpdateAvailableModal';
import { VttCanvas } from './components/VttCanvas';
import { useSaves } from './hooks/useSaves';
import { useUpdater } from './hooks/useUpdater';
import i18n from './i18n';
// useSession is mounted inside ChatPanel so the retry-bar can call refetch.
import { useStore } from './state/useStore';
import { Icons } from './ui/Icons';

const PROVIDER_LABELS: Record<string, string> = {
  anthropic: 'Anthropic',
  'openai-compat': 'OpenAI compatible',
  'local-mistralrs': 'Local mistralrs',
};

function getProviderModel(state: ReturnType<typeof useStore.getState>): string {
  const active = state.settings.activeProvider;
  const cfg = state.settings.providers[active];
  if (cfg === null) return '—';
  return 'model' in cfg ? cfg.model : cfg.modelPath;
}

/**
 * Tiny "Saved at HH:MM" toast that fades in for ~2.5s after every successful
 * quick save. Watches the saves slice's `lastQuickSaveAt` ISO string - bumping
 * that field is what actually triggers the toast (the keyboard handler does
 * not need to call into the toast directly).
 */
function QuickSaveToast({ lastQuickSaveAt }: { lastQuickSaveAt: string | null }) {
  const { t } = useTranslation('saves');
  const [visible, setVisible] = useState(false);
  useEffect(() => {
    if (!lastQuickSaveAt) return;
    setVisible(true);
    const handle = window.setTimeout(() => setVisible(false), 2500);
    return () => window.clearTimeout(handle);
  }, [lastQuickSaveAt]);
  if (!visible || !lastQuickSaveAt) return null;
  const time = new Date(lastQuickSaveAt).toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
  });
  return (
    <div className="dm-saves-toast" role="status" aria-live="polite">
      {t('saved_now')} - {time}
    </div>
  );
}

async function tauriWindowAction(action: 'minimize' | 'toggleMaximize' | 'close'): Promise<void> {
  try {
    const win = getCurrentWindow();
    if (action === 'minimize') await win.minimize();
    else if (action === 'toggleMaximize') await win.toggleMaximize();
    else await win.close();
  } catch {
    // Outside Tauri (e.g. vite dev preview) the API throws — ignore silently.
  }
}

function App() {
  const { t } = useTranslation('common');
  const { t: tCombat } = useTranslation('combat');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [localModeOpen, setLocalModeOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);

  const uiLanguage = useStore((s) => s.settings.uiLanguage);
  const chatPanelWidth = useStore((s) => s.settings.chatPanelWidth);
  const activeProvider = useStore((s) => s.settings.activeProvider);
  const activeProviderConfig = useStore((s) => s.settings.providers[s.settings.activeProvider]);
  const combatActive = useStore((s) => s.combat.active);
  const combatTokens = useStore((s) => s.combat.tokens);
  const combatOrder = useStore((s) => s.combat.initiativeOrder);
  const combatRound = useStore((s) => s.combat.round);
  const currentTurnId = useStore((s) => s.combat.currentTurnId);
  const journalEntries = useStore((s) => s.journal.entries);
  const journalOpen = useStore((s) => s.journal.isOpen);
  const closeJournal = useStore((s) => s.journal.close);
  const openJournal = useStore((s) => s.journal.open);
  const npcRecords = useStore((s) => s.npcs.records);
  const npcsOpen = useStore((s) => s.npcs.isOpen);
  const closeNpcs = useStore((s) => s.npcs.close);
  const openNpcs = useStore((s) => s.npcs.open);
  const toolEntries = useStore((s) => s.toolLog.entries);
  const currentScene = useStore((s) => s.session.currentScene);
  const onboardingCompleted = useStore((s) => s.onboarding.completed);
  const savesOpen = useStore((s) => s.saves.isOpen);
  const lastQuickSaveAt = useStore((s) => s.saves.lastQuickSaveAt);
  const { open: openSaves, quickSave } = useSaves();

  const { pending: pendingUpdate, dismiss: dismissUpdate } = useUpdater();

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void initBackendListener().then((u) => {
      if (cancelled) u();
      else unlisten = u;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (i18n.language !== uiLanguage) {
      void i18n.changeLanguage(uiLanguage);
    }
  }, [uiLanguage]);

  // Apply the persisted chat panel width to the `--chat-width` CSS var on
  // the root element so the grid track in shell.css picks it up on boot
  // (and after rehydration, when the persisted value differs from the
  // default). The drag handle also writes to this var directly during a
  // drag for flicker-free resizing; this effect is the source of truth on
  // mount and whenever the slice changes via keyboard or programmatic set.
  useEffect(() => {
    document.documentElement.style.setProperty('--chat-width', `${chatPanelWidth}px`);
  }, [chatPanelWidth]);

  // Dev-mode keyboard shortcuts. The Saves modal moves under Ctrl+Shift+S
  // because plain Ctrl+S is reserved for the quick-save action (no modal).
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      // Ctrl+S (no shift) -> quick save. Run on the active session and
      // pop the saved-now toast via the slice's `lastQuickSaveAt` clock.
      if (e.ctrlKey && !e.shiftKey && (e.key === 's' || e.key === 'S')) {
        e.preventDefault();
        void quickSave();
        return;
      }
      if (!(e.ctrlKey && e.shiftKey)) return;
      if (e.key === 'M' || e.key === 'm') {
        e.preventDefault();
        setLocalModeOpen((prev) => !prev);
      } else if (e.key === 'I' || e.key === 'i') {
        e.preventDefault();
        setInspectorOpen((prev) => !prev);
      } else if (e.key === 'S' || e.key === 's') {
        e.preventDefault();
        openSaves();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [openSaves, quickSave]);

  const providerLabel = PROVIDER_LABELS[activeProvider] ?? activeProvider;
  const providerStatus = activeProviderConfig === null ? 'error' : 'connected';
  const modelLabel = getProviderModel(useStore.getState());
  const npcList = Object.values(npcRecords);

  return (
    <div className="dm-app">
      <header className="dm-titlebar" data-tauri-drag-region>
        <div className="dm-titlebar-left">
          <div className="dm-app-mark">
            <div className="dm-app-mark-glyph">
              <Icons.D20 size={14} />
            </div>
            <span className="dm-app-mark-name">DUNGEON MASTER AI</span>
          </div>
          <div className="dm-titlebar-divider" />
          <div className="dm-titlebar-campaign">
            <Icons.Book size={12} />
            <span>{t('campaign_default')}</span>
          </div>
        </div>

        <div className="dm-titlebar-center">
          <ScenePill scene={currentScene} />
        </div>

        <div className="dm-titlebar-right">
          <button
            type="button"
            className="dm-btn-tb"
            onClick={openSaves}
            aria-label={t('saves')}
            title={t('saves')}
          >
            <Icons.Save size={13} />
            <span>{t('saves')}</span>
          </button>
          <button
            type="button"
            className="dm-btn-tb"
            onClick={openJournal}
            aria-label={t('journal')}
            title={t('journal')}
          >
            <Icons.Scroll size={13} />
            <span>{t('journal')}</span>
          </button>
          <button
            type="button"
            className="dm-btn-tb"
            onClick={openNpcs}
            aria-label={t('npcs')}
            title={t('npcs')}
          >
            <Icons.User size={13} />
            <span>{t('npcs')}</span>
          </button>
          <div className="dm-titlebar-divider" />
          <button
            type="button"
            className="dm-btn-tb dm-btn-tb-icon"
            onClick={() => setSettingsOpen(true)}
            aria-label={t('settings')}
            title={t('settings')}
          >
            <Icons.Settings size={14} />
          </button>
          <div className="dm-window-controls">
            <button
              type="button"
              className="dm-window-ctrl"
              onClick={() => void tauriWindowAction('minimize')}
              aria-label={t('minimize')}
              title={t('minimize')}
            >
              <Icons.Minimize size={10} />
            </button>
            <button
              type="button"
              className="dm-window-ctrl"
              onClick={() => void tauriWindowAction('toggleMaximize')}
              aria-label={t('maximize')}
              title={t('maximize')}
            >
              <Icons.Square size={10} />
            </button>
            <button
              type="button"
              className="dm-window-ctrl dm-window-close"
              onClick={() => void tauriWindowAction('close')}
              aria-label={t('close')}
              title={t('close')}
            >
              <Icons.X size={10} />
            </button>
          </div>
        </div>
      </header>

      <main className="dm-vtt-panel">
        <VttCanvas />
        {combatActive && (
          <InitiativeTracker
            tokens={combatTokens}
            order={combatOrder}
            round={combatRound}
            activeTokenId={currentTurnId}
          />
        )}
        {combatActive && (
          <ActionBar
            actionUsed={false}
            bonusUsed={false}
            reactionUsed={false}
            movementFt={30}
            speedFt={30}
          />
        )}
      </main>

      <aside className="dm-chat-panel" aria-label={tCombat('initiative_tracker')}>
        <ChatResizer />
        <ChatPanel />
      </aside>

      <StatusBar provider={providerLabel} model={modelLabel} status={providerStatus} />

      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <LocalModeModal open={localModeOpen} onClose={() => setLocalModeOpen(false)} />
      {pendingUpdate && (
        <UpdateAvailableModal
          version={pendingUpdate.version}
          notes={pendingUpdate.notes}
          onLater={dismissUpdate}
          onUpdate={() => {
            void pendingUpdate.install().finally(dismissUpdate);
          }}
        />
      )}
      {journalOpen && <JournalViewer entries={journalEntries} onClose={closeJournal} />}
      {npcsOpen && <NpcMemoryGrid npcs={npcList} onClose={closeNpcs} />}
      {savesOpen && <SavesScreen />}
      <QuickSaveToast lastQuickSaveAt={lastQuickSaveAt} />
      <ToolInspectorDrawer
        entries={toolEntries}
        isOpen={inspectorOpen}
        onClose={() => setInspectorOpen(false)}
      />
      {!onboardingCompleted && <Onboarding />}
    </div>
  );
}

export default App;
