import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useShallow } from 'zustand/react/shallow';
import { initBackendListener } from './api/client';
import { ActionBar } from './components/ActionBar';
import { CharacterSheet } from './components/CharacterSheet';
import { CharacterWizard } from './components/CharacterWizard';
import { CharFab } from './components/CharFab';
import { ChatPanel } from './components/ChatPanel';
import { ChatResizer } from './components/ChatResizer';
import { ErrorBoundary } from './components/ErrorBoundary';
import { InitiativeTracker } from './components/InitiativeTracker';
import { JournalViewer } from './components/JournalViewer';
import { LocalModeModal } from './components/LocalModeModal';
import { NpcMemoryGrid } from './components/NpcMemoryGrid';
import { Onboarding } from './components/Onboarding';
import { PreflightModal } from './components/PreflightModal';
import { ProviderMigrationBanner } from './components/ProviderMigrationBanner';
import { SavesScreen } from './components/SavesScreen';
import { ScenePill } from './components/ScenePill';
import { SceneTransitionOverlay } from './components/SceneTransitionOverlay';
import { SettingsModal } from './components/SettingsModal';
import { SplashOverlay } from './components/SplashOverlay';
import { StatusBar } from './components/StatusBar';
import { ToolInspectorDrawer } from './components/ToolInspectorDrawer';
import { UpdateAvailableModal } from './components/UpdateAvailableModal';
import { VideoDisabledToast } from './components/VideoDisabledToast';
import { VttCanvas } from './components/VttCanvas';
import { useAgentTurn } from './hooks/useAgentTurn';
import { useHydrated } from './hooks/useHydrated';
import { useSaves } from './hooks/useSaves';
import { useUpdater } from './hooks/useUpdater';
import i18n from './i18n';
import type { PreflightInput, PreflightStatus } from './lib/preflight';
// useSession is mounted inside ChatPanel so the retry-bar can call refetch.
import { isPreflightDismissed, runPreflight } from './lib/preflight';
import { useStore } from './state/useStore';
import { Icons } from './ui/Icons';

function getProviderModel(state: ReturnType<typeof useStore.getState>): string {
  const active = state.settings.activeProvider;
  const cfg = state.settings.providers[active];
  if (cfg === null) return '-';
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

/**
 * Error toast for a failed save/overwrite/delete. Reads the saves slice's
 * `lastSaveError`; auto-dismisses after a few seconds and on click. [F3]
 */
function SaveErrorToast() {
  const { t } = useTranslation('saves');
  const lastSaveError = useStore((s) => s.saves.lastSaveError);
  const clear = useStore((s) => s.saves.setLastSaveError);
  useEffect(() => {
    if (!lastSaveError) return;
    const handle = window.setTimeout(() => clear(null), 5000);
    return () => window.clearTimeout(handle);
  }, [lastSaveError, clear]);
  if (!lastSaveError) return null;
  return (
    <button
      type="button"
      className="dm-saves-toast dm-saves-toast-error"
      aria-live="assertive"
      onClick={() => clear(null)}
    >
      {t('save_failed')}: {lastSaveError}
    </button>
  );
}

async function tauriWindowAction(action: 'minimize' | 'toggleMaximize' | 'close'): Promise<void> {
  try {
    const win = getCurrentWindow();
    if (action === 'minimize') await win.minimize();
    else if (action === 'toggleMaximize') await win.toggleMaximize();
    else await win.close();
  } catch (err) {
    // Outside Tauri (e.g. vite dev preview) the API throws. Inside Tauri
    // a failure usually means a missing capability; log so it is visible
    // in the dev tools console.
    console.warn(`window action ${action} failed`, err);
  }
}

function App() {
  const { t } = useTranslation('common');
  const { t: tSettings } = useTranslation('settings');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsInitialTab, setSettingsInitialTab] = useState<'chat' | 'image' | 'video'>('chat');
  const [localModeOpen, setLocalModeOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [characterSheetOpen, setCharacterSheetOpen] = useState(false);
  const [wizardReopen, setWizardReopen] = useState(false);

  const uiLanguage = useStore((s) => s.settings.uiLanguage);
  const chatPanelWidth = useStore((s) => s.settings.chatPanelWidth);
  const activeProvider = useStore((s) => s.settings.activeProvider);
  const activeProviderConfig = useStore((s) => s.settings.providers[s.settings.activeProvider]);
  const combatActive = useStore((s) => s.combat.active);
  // ActionBar posts combat-action intents to the DM through the agent turn. [F1]
  const { send: sendAgentTurn } = useAgentTurn();
  const combatTokens = useStore((s) => s.combat.tokens);
  const combatOrder = useStore((s) => s.combat.initiativeOrder);
  const combatRound = useStore((s) => s.combat.round);
  const currentTurnId = useStore((s) => s.combat.currentTurnId);
  const setCurrentTurn = useStore((s) => s.combat.setCurrentTurn);
  // Resolve the PC token id by matching pc.name to a token name (same logic as
  // CombatToken's isPcToken). Fall back to null when no match so the ActionBar
  // stays hidden rather than incorrectly shown on a lookup miss. [W1.6]
  const pcName = useStore((s) => s.pc.name);
  const pcTokenId = combatTokens.find((t) => pcName !== null && t.name === pcName)?.id ?? null;
  // Show the ActionBar only during the player's own turn. When currentTurnId is
  // null or no PC token is found we keep the bar hidden; a lookup miss does not
  // force-show it. [W1.6]
  const showActionBar = combatActive && pcTokenId !== null && currentTurnId === pcTokenId;
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
  // Gate first-run UI on persist hydration: before the async rehydrate finishes
  // the store still holds slice defaults (onboarding.completed === false), which
  // would flash the Onboarding modal on every launch. (Audit blocker 1.)
  const hydrated = useHydrated();

  // Preflight: read the settings fields needed by runPreflight in one selector
  // so we only re-render when these specific fields change.
  // useShallow prevents re-renders when the object identity changes but values
  // are the same (Zustand v5 uses Object.is by default).
  const preflightSettings = useStore(
    useShallow(
      (s): PreflightInput => ({
        activeProvider: s.settings.activeProvider,
        providers: s.settings.providers,
        imageEnabled: s.settings.imageEnabled,
        imagePreset: s.settings.imagePreset,
        replicateApiKey: s.settings.replicateApiKey,
        videoEnabled: s.settings.videoEnabled,
        videoMode: s.settings.videoMode,
      }),
    ),
  );
  // A local flag lets the user dismiss for this session without a dontAskAgain
  // checkbox click. It is reset when the component unmounts (app reload).
  const [preflightDismissedThisSession, setPreflightDismissedThisSession] = useState(false);

  const pcHeroClass = useStore((s) => s.pc.heroClass);
  const savesOpen = useStore((s) => s.saves.isOpen);
  const lastQuickSaveAt = useStore((s) => s.saves.lastQuickSaveAt);
  const { open: openSaves, quickSave } = useSaves();

  const { pending: pendingUpdate, dismiss: dismissUpdate } = useUpdater();

  const handleOpenChatSettings = useCallback(() => {
    setSettingsInitialTab('chat');
    setSettingsOpen(true);
  }, []);

  const handleOpenImageSettings = useCallback(() => {
    setSettingsInitialTab('image');
    setSettingsOpen(true);
  }, []);

  const handleOpenVideoSettings = useCallback(() => {
    setSettingsInitialTab('video');
    setSettingsOpen(true);
  }, []);

  const handleInitiativeSelect = useCallback(
    (tokenId: string) => {
      if (combatActive) {
        setCurrentTurn(tokenId);
      }
    },
    [combatActive, setCurrentTurn],
  );

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
      // F11 -> toggle fullscreen. [M11 G3]
      if (e.key === 'F11') {
        e.preventDefault();
        const win = getCurrentWindow();
        void win.isFullscreen().then((fs) => win.setFullscreen(!fs));
        return;
      }
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

  const providerLabel = tSettings(
    `provider_${activeProvider.replace('-', '_')}` as
      | 'provider_openai_compat'
      | 'provider_local_mistralrs',
  );
  const providerStatus = activeProviderConfig === null ? 'error' : 'connected';
  const modelLabel = getProviderModel(useStore.getState());
  const imageEnabled = useStore((s) => s.settings.imageEnabled);
  const imagePreset = useStore((s) => s.settings.imagePreset);
  const videoEnabled = useStore((s) => s.settings.videoEnabled);
  const videoMode = useStore((s) => s.settings.videoMode);
  const sceneTransitionsEnabled = useStore((s) => s.settings.sceneTransitionsEnabled);
  const npcList = Object.values(npcRecords);

  // StatusBar "saved" chip: derive minutes-since-last-quicksave from the saves
  // slice. Recomputed on every render that touches lastQuickSaveAt; not a live
  // ticking clock, which is fine for an at-a-glance footer chip. [F4]
  let savedAgo: { minutes: number } | 'now' | null = null;
  if (lastQuickSaveAt) {
    const mins = Math.floor((Date.now() - new Date(lastQuickSaveAt).getTime()) / 60000);
    savedAgo = mins < 1 ? 'now' : { minutes: mins };
  }

  // Compute preflight status - only relevant after onboarding is completed.
  const preflightStatus: PreflightStatus = onboardingCompleted
    ? runPreflight(preflightSettings)
    : 'ok';
  const showPreflight =
    onboardingCompleted &&
    preflightStatus !== 'ok' &&
    !preflightDismissedThisSession &&
    !isPreflightDismissed(preflightStatus);

  return (
    <ErrorBoundary level="top">
      <ProviderMigrationBanner />
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

        <ErrorBoundary level="section">
          <main className="dm-vtt-panel">
            <VttCanvas />
            {combatActive && (
              <InitiativeTracker
                tokens={combatTokens}
                order={combatOrder}
                round={combatRound}
                activeTokenId={currentTurnId}
                onSelect={handleInitiativeSelect}
              />
            )}
            {showActionBar && (
              <ActionBar
                onIntent={(text) => {
                  void sendAgentTurn(text);
                }}
              />
            )}
            <CharFab
              onOpen={() => setCharacterSheetOpen(true)}
              onOpenWizard={() => setWizardReopen(true)}
            />
          </main>

          <aside className="dm-chat-panel" aria-label={t('chat_panel')}>
            <ChatResizer />
            <ChatPanel />
          </aside>
        </ErrorBoundary>

        <StatusBar
          provider={providerLabel}
          model={modelLabel}
          status={providerStatus}
          savedAgo={savedAgo}
          image={{ enabled: imageEnabled, label: imageEnabled ? imagePreset : t('modality_off') }}
          video={{ enabled: videoEnabled, label: videoEnabled ? videoMode : t('modality_off') }}
          onOpenSettings={(tab) => {
            setSettingsInitialTab(tab);
            setSettingsOpen(true);
          }}
        />

        <ErrorBoundary level="overlay">
          <SettingsModal
            open={settingsOpen}
            onClose={() => setSettingsOpen(false)}
            initialTab={settingsInitialTab}
            onRequestCharacterRecreate={() => {
              setSettingsOpen(false);
              setWizardReopen(true);
            }}
          />
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
          <SaveErrorToast />
          <ToolInspectorDrawer
            entries={toolEntries}
            isOpen={inspectorOpen}
            onClose={() => setInspectorOpen(false)}
          />
          <CharacterSheet open={characterSheetOpen} onClose={() => setCharacterSheetOpen(false)} />
          {hydrated && !onboardingCompleted && (
            <Onboarding
              onExitToWizard={() => setWizardReopen(true)}
              onComplete={(preset) => {
                // The manual preset configures nothing, so it would land on the
                // blocking PreflightModal (missing_chat). Route the user straight
                // to the Chat settings tab instead and skip the blocking modal
                // for this session. [E2]
                if (preset === 'manual') {
                  setPreflightDismissedThisSession(true);
                  handleOpenChatSettings();
                }
              }}
            />
          )}
          {showPreflight && (
            <PreflightModal
              status={preflightStatus as Exclude<PreflightStatus, 'ok'>}
              onFinishSetup={() => {
                setPreflightDismissedThisSession(true);
                if (preflightStatus === 'missing_chat') {
                  handleOpenChatSettings();
                } else if (preflightStatus === 'missing_image') {
                  handleOpenImageSettings();
                } else {
                  handleOpenVideoSettings();
                }
              }}
              onDismiss={() => setPreflightDismissedThisSession(true)}
            />
          )}
          {wizardReopen && (
            <CharacterWizard
              mode={pcHeroClass ? 'edit' : 'initial'}
              onClose={() => setWizardReopen(false)}
              onOpenImageSettings={handleOpenImageSettings}
              hidden={settingsOpen}
            />
          )}
          <SceneTransitionOverlay />
          <VideoDisabledToast
            videoEnabled={videoEnabled}
            sceneTransitionsEnabled={sceneTransitionsEnabled}
            sceneName={currentScene?.name ?? null}
            onOpenVideoSettings={handleOpenVideoSettings}
          />
          <SplashOverlay />
        </ErrorBoundary>
      </div>
    </ErrorBoundary>
  );
}

export default App;
