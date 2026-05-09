import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { initBackendListener } from './api/client';
import { ActionBar } from './components/ActionBar';
import { ChatPanel } from './components/ChatPanel';
import { InitiativeTracker } from './components/InitiativeTracker';
import { JournalViewer } from './components/JournalViewer';
import { LocalModeModal } from './components/LocalModeModal';
import { NpcMemoryGrid } from './components/NpcMemoryGrid';
import { SettingsModal } from './components/SettingsModal';
import { StatusBar } from './components/StatusBar';
import { ToolInspectorDrawer } from './components/ToolInspectorDrawer';
import { UpdateAvailableModal } from './components/UpdateAvailableModal';
import { VttCanvas } from './components/VttCanvas';
import { useSession } from './hooks/useSession';
import { useUpdater } from './hooks/useUpdater';
import i18n from './i18n';
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

  const { pending: pendingUpdate, dismiss: dismissUpdate } = useUpdater();
  useSession();

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

  // Dev-mode keyboard shortcuts: Ctrl+Shift+M opens local-mode config,
  // Ctrl+Shift+I toggles the tool-call inspector. The local-mode UI moves
  // into Settings -> Provider tab in a later milestone; the shortcut keeps
  // the configuration reachable in the meantime.
  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if (!(e.ctrlKey && e.shiftKey)) return;
      if (e.key === 'M' || e.key === 'm') {
        e.preventDefault();
        setLocalModeOpen((prev) => !prev);
      } else if (e.key === 'I' || e.key === 'i') {
        e.preventDefault();
        setInspectorOpen((prev) => !prev);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

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
          {/* Scene-pill placeholder; populated when scene state lands in the chat slice. */}
        </div>

        <div className="dm-titlebar-right">
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
      <ToolInspectorDrawer
        entries={toolEntries}
        isOpen={inspectorOpen}
        onClose={() => setInspectorOpen(false)}
      />
    </div>
  );
}

export default App;
