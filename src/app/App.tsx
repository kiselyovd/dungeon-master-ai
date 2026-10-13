import { getCurrentWindow } from '@tauri-apps/api/window';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getPendingAgentRecoveryInput } from '../api/agentPending';
import { fetchAgentRecovery } from '../api/agentRecovery';
import { ActionBar } from '../components/ActionBar';
import { ActorPortrait } from '../components/ActorPortrait';
import { CharacterSheet } from '../components/CharacterSheet';
import { CharacterWizard } from '../components/CharacterWizard';
import { CharFab } from '../components/CharFab';
import { ChatPanel } from '../components/ChatPanel';
import { ChatResizer } from '../components/ChatResizer';
import { ErrorBoundary } from '../components/ErrorBoundary';
import { HeroContext } from '../components/HeroContext';
import { InitiativeTracker } from '../components/InitiativeTracker';
import { JournalViewer } from '../components/JournalViewer';
import { LocalModeModal } from '../components/LocalModeModal';
import { NpcMemoryGrid } from '../components/NpcMemoryGrid';
import { Onboarding } from '../components/Onboarding';
import { ProviderMigrationBanner } from '../components/ProviderMigrationBanner';
import { RecoveryPanel } from '../components/RecoveryPanel';
import { SavesScreen } from '../components/SavesScreen';
import { ScenePill } from '../components/ScenePill';
import { SceneTransitionOverlay } from '../components/SceneTransitionOverlay';
import { SettingsModal } from '../components/SettingsModal';
import { SplashOverlay } from '../components/SplashOverlay';
import { StatusBar } from '../components/StatusBar';
import { ToolInspectorDrawer } from '../components/ToolInspectorDrawer';
import { UpdateAvailableModal } from '../components/UpdateAvailableModal';
import { VideoDisabledToast } from '../components/VideoDisabledToast';
import { VttCanvas } from '../components/VttCanvas';
import { AdventureContent } from '../features/adventure/AdventurePanel';
import { useAdventure } from '../features/adventure/useAdventure';
import { CampaignCreate } from '../features/campaigns/CampaignCreate';
import { CampaignHeroChoice } from '../features/campaigns/CampaignHeroChoice';
import { CampaignHome } from '../features/campaigns/CampaignHome';
import { resolveAppRoute } from '../features/campaigns/model/route';
import { useCampaigns } from '../features/campaigns/useCampaigns';
import { isPlayerToken } from '../features/combat/model/selectors';
import { canCast } from '../features/combat/model/spellAvailability';
import { SpellCastDialog } from '../features/combat/SpellCastDialog';
import { useCombatCommands } from '../features/combat/useCombatCommands';
import { useCombatSelection } from '../features/combat/useCombatSelection';
import { WeaponAttackDialog } from '../features/combat/WeaponAttackDialog';
import { GameTable } from '../features/game-table';
import { useLegacyCampaignPortrait } from '../features/media/useLegacyCampaignPortrait';
import { useFirstEntry } from '../features/onboarding/useFirstEntry';
import { type RecoveryPorts, useRecovery } from '../features/recovery/useRecovery';
import { RestDialog } from '../features/rest/RestDialog';
import { useSaves } from '../features/saves/useSaves';
import { useHydrated } from '../hooks/useHydrated';
import { useLocalRuntimeStatus } from '../hooks/useLocalRuntimeStatus';
import { useUpdater } from '../hooks/useUpdater';
import { selectedLocalModelIsRunning } from '../state/localMode';
// useSession is mounted inside ChatPanel so the retry-bar can call refetch.
import { loadRestOutbox } from '../state/restOutbox';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';
import { Icons } from '../ui/Icons';
import { LanguageSegmentedControl } from '../ui/LanguageSegmentedControl';
import { AppOverlays } from './AppOverlays';
import { AppShell } from './AppShell';
import { GameTableExperience } from './GameTableExperience';
import { RoomExperience } from './RoomExperience';
import { exitRoomToLocalCampaign } from './roomExit';
import { soloTable } from './tableAdapters';
import { useAgentTurn } from './useAgentTurn';
import { useAppShellController } from './useAppShellController';

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

interface CampaignTitleBarProps {
  language: 'en' | 'ru';
  campaignTitle?: string;
  onLanguageChange: (language: 'en' | 'ru') => void;
  onOpenSettings: () => void;
  onOpenRoom?: () => void;
  hideSettings?: boolean;
}

function CampaignTitleBar({
  language,
  campaignTitle,
  onLanguageChange,
  onOpenSettings,
  onOpenRoom,
  hideSettings,
}: CampaignTitleBarProps) {
  const { t } = useTranslation('common');
  const { t: tCampaigns } = useTranslation('campaigns');

  return (
    <header className="dm-titlebar" data-tauri-drag-region>
      <div className="dm-titlebar-left">
        <div className="dm-app-mark">
          <div className="dm-app-mark-glyph" data-testid="brand-crest">
            <Icons.Crest size={18} strokeWidth={1.35} />
          </div>
          <span className="dm-app-mark-name">DUNGEON MASTER AI</span>
        </div>
        <div className="dm-titlebar-divider" />
        <div className="dm-titlebar-campaign">
          <Icons.Book size={12} />
          <span>{campaignTitle ?? tCampaigns('library_eyebrow')}</span>
        </div>
      </div>

      <div className="dm-titlebar-center" />

      <div className="dm-titlebar-right">
        <LanguageSegmentedControl
          value={language}
          onChange={onLanguageChange}
          ariaLabel={t('language')}
        />
        {onOpenRoom && (
          <button type="button" className="dm-btn-tb" onClick={onOpenRoom}>
            {tCampaigns('coop')}
          </button>
        )}
        {!hideSettings && (
          <button
            type="button"
            className="dm-btn-tb dm-btn-tb-icon"
            onClick={onOpenSettings}
            aria-label={t('settings')}
            title={t('settings')}
          >
            <Icons.Settings size={14} />
          </button>
        )}
        <div className="dm-window-controls">
          <button
            type="button"
            className="dm-window-ctrl"
            data-testid="window-minimize"
            onClick={() => void tauriWindowAction('minimize')}
            aria-label={t('minimize')}
            title={t('minimize')}
          >
            <Icons.Minimize size={10} />
          </button>
          <button
            type="button"
            className="dm-window-ctrl"
            data-testid="window-maximize"
            onClick={() => void tauriWindowAction('toggleMaximize')}
            aria-label={t('maximize')}
            title={t('maximize')}
          >
            <Icons.Square size={10} />
          </button>
          <button
            type="button"
            className="dm-window-ctrl dm-window-close"
            data-testid="window-close"
            onClick={() => void tauriWindowAction('close')}
            aria-label={t('close')}
            title={t('close')}
          >
            <Icons.X size={10} />
          </button>
        </div>
      </div>
    </header>
  );
}

function App() {
  const [roomOpen, setRoomOpen] = useState(false);
  const [customHeroCampaign, setCustomHeroCampaign] = useState<string | null>(null);
  const { t } = useTranslation('common');
  const { t: tAdventure } = useTranslation('adventure');
  const { t: tCampaigns } = useTranslation('campaigns');
  const { t: tSettings } = useTranslation('settings');
  const { t: tCharacter } = useTranslation('character');
  const firstEntry = useStore((state) => state.onboarding.entry);
  const localChatReady = useStore((state) => selectedLocalModelIsRunning(state.localMode));
  const uiLanguage = useStore((s) => s.settings.uiLanguage);
  const readingFontSize = useStore((s) => s.settings.readingFontSize);
  const chatPanelWidth = useStore((s) => s.settings.chatPanelWidth);
  const activeProvider = useStore((s) => s.settings.activeProvider);
  const activeProviderConfig = useStore((s) => s.settings.providers[s.settings.activeProvider]);
  const combatActive = useStore((s) => s.combat.active);
  // ActionBar posts combat-action intents to the DM through the agent turn. [F1]
  const agentTurn = useAgentTurn();
  const sendAgentTurn = agentTurn.send;
  const agentStreaming = useStore((s) => s.chat.isStreaming);
  const combatTokens = useStore((s) => s.combat.tokens);
  const combatOrder = useStore((s) => s.combat.initiativeOrder);
  const combatRound = useStore((s) => s.combat.round);
  const currentTurnId = useStore((s) => s.combat.currentTurnId);
  const combatCommands = useCombatCommands();
  const combatAuthored = useStore((s) => s.combat.authored);
  const [weaponAttackScope, setWeaponAttackScope] = useState<string | null>(null);
  const [spellCastScope, setSpellCastScope] = useState<string | null>(null);
  const [restScope, setRestScope] = useState<string | null>(null);
  const combatEncounterId = useStore((s) => s.combat.encounterId);
  const combatRevision = useStore((s) => s.combat.revision);
  const sessionGeneration = useStore((s) => s.session.transitionGeneration);
  const pcName = useStore((s) => s.pc.name);
  const pcCharacterId = useStore((s) => s.pc.characterId);
  const pcTokenId =
    combatTokens.find((token) => isPlayerToken(token, pcCharacterId, pcName))?.id ?? null;
  // Show the ActionBar only during the player's own turn. When currentTurnId is
  // null or no PC token is found we keep the bar hidden; a lookup miss does not
  // force-show it. [W1.6]
  const showActionBar = combatActive && pcTokenId !== null && currentTurnId === pcTokenId;
  const ownedCombatToken = combatTokens.find((token) => token.id === pcTokenId);
  const currentAttackScope = `${combatEncounterId}:${combatRevision}:${ownedCombatToken?.id}:${sessionGeneration}`;
  const combatSelection = useCombatSelection(
    `${combatEncounterId}:${sessionGeneration}:${pcCharacterId}`,
    combatTokens,
  );
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
  useLocalRuntimeStatus(
    hydrated &&
      onboardingCompleted &&
      firstEntry !== 'guest' &&
      !roomOpen &&
      activeProvider === 'local-mistralrs',
  );
  const campaignsFeature = useCampaigns();
  const enterRooms = useCallback(() => {
    useStore.getState().chat.abort();
    setRoomOpen(true);
    void campaignsFeature.refresh();
  }, [campaignsFeature.refresh]);
  const adventureFeature = useAdventure();
  const [adventureOpen, setAdventureOpen] = useState(false);
  const closeAdventure = useCallback(() => setAdventureOpen(false), []);
  const [tableContextId, setTableContextId] = useState<string | null>(null);
  const selectedTableContext = journalOpen
    ? 'journal'
    : npcsOpen
      ? 'npcs'
      : adventureOpen
        ? 'adventure'
        : tableContextId;
  const selectTableContext = (id: string | null) => {
    closeJournal();
    closeNpcs();
    closeAdventure();
    setTableContextId(id);
  };
  const persistedCampaignId = useStore((s) => s.session.activeCampaignId);
  const currentRestScope = `${persistedCampaignId}:${pcCharacterId}:${sessionGeneration}`;
  useEffect(() => {
    if (persistedCampaignId && pcCharacterId && loadRestOutbox(persistedCampaignId))
      setRestScope(currentRestScope);
  }, [persistedCampaignId, pcCharacterId, currentRestScope]);
  const setUiLanguage = useStore((s) => s.settings.setUiLanguage);
  const [campaignCreateOpen, setCampaignCreateOpen] = useState(false);

  // Preflight: read the settings fields needed by runPreflight in one selector
  // so we only re-render when these specific fields change.
  const pcHeroClass = useStore((s) => s.pc.heroClass);
  const savesOpen = useStore((s) => s.saves.isOpen);
  const lastQuickSaveAt = useStore((s) => s.saves.lastQuickSaveAt);
  const { open: openSaves, quickSave } = useSaves();
  const {
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
  } = useAppShellController({
    uiLanguage,
    chatPanelWidth,
    quickSave,
    openSaves,
    enabled: hydrated && !roomOpen,
  });

  useFirstEntry(hydrated, firstEntry, () => setRoomOpen(true));

  const { pending: pendingUpdate, dismiss: dismissUpdate } = useUpdater();

  const handleOpenChatSettings = useCallback(() => {
    setSettingsInitialTab('chat');
    setSettingsOpen(true);
  }, [setSettingsInitialTab, setSettingsOpen]);

  const handleOpenImageSettings = useCallback(() => {
    setSettingsInitialTab('image');
    setSettingsOpen(true);
  }, [setSettingsInitialTab, setSettingsOpen]);

  const handleOpenVideoSettings = useCallback(() => {
    setSettingsInitialTab('video');
    setSettingsOpen(true);
  }, [setSettingsInitialTab, setSettingsOpen]);

  const providerLabel = tSettings(
    `provider_${activeProvider.replace('-', '_')}` as
      | 'provider_openai_compat'
      | 'provider_local_mistralrs',
  );
  const needsModelSetup =
    activeProviderConfig === null || (activeProvider === 'local-mistralrs' && !localChatReady);
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

  useEffect(() => {
    if (
      hydrated &&
      onboardingCompleted &&
      !roomOpen &&
      firstEntry !== 'guest' &&
      campaignsFeature.active === null
    ) {
      if (persistedCampaignId) {
        void campaignsFeature.activate(persistedCampaignId);
      } else {
        void campaignsFeature.refresh();
      }
    }
  }, [
    campaignsFeature.active,
    campaignsFeature.activate,
    campaignsFeature.refresh,
    hydrated,
    onboardingCompleted,
    persistedCampaignId,
    roomOpen,
    firstEntry,
  ]);

  const campaignRoute = campaignCreateOpen
    ? ({ kind: 'campaign-create', draftId: 'local' } as const)
    : resolveAppRoute({
        hydrated,
        onboardingComplete: onboardingCompleted,
        active: campaignsFeature.active
          ? {
              id: campaignsFeature.active.id,
              sessionId: campaignsFeature.active.last_session_id,
              heroReady: pcHeroClass !== null,
              worldReady:
                campaignsFeature.active.seed.kind === 'bundled'
                  ? useStore.getState().session.adventure?.state.package_id ===
                    campaignsFeature.active.seed.adventure_id
                  : Boolean(currentScene),
            }
          : null,
      });

  const recoveryPorts = useMemo<RecoveryPorts>(
    () => ({
      projection: (scope, signal) => fetchAgentRecovery(scope.campaignId, scope.sessionId, signal),
      input: (scope, signal) =>
        getPendingAgentRecoveryInput(scope.campaignId, scope.sessionId, signal),
      prepare: agentTurn.prepareRecovery,
      resume: agentTurn.resumeFrom,
    }),
    [agentTurn.prepareRecovery, agentTurn.resumeFrom],
  );
  const recovery = useRecovery(
    !roomOpen && campaignRoute.kind === 'game-table' && pcCharacterId
      ? {
          campaignId: campaignRoute.campaignId,
          sessionId: campaignRoute.sessionId,
          actorId: pcCharacterId,
          generation: sessionGeneration,
        }
      : null,
    recoveryPorts,
    agentStreaming,
  );
  useLegacyCampaignPortrait(hydrated && !roomOpen && !recovery.blocked);
  const recoveryNotice = (
    // Recovery must settle before an automatic appearance migration can write.
    <RecoveryPanel
      view={recovery.view}
      status={recovery.status}
      busy={agentStreaming}
      onResume={() => void recovery.resume()}
      onRetry={() => void recovery.retry()}
      onConfigure={() => setSettingsOpen(true)}
    />
  );

  if (!hydrated) {
    return (
      <ErrorBoundary level="top">
        <AppShell readingFontSize={readingFontSize}>
          <SplashOverlay />
        </AppShell>
      </ErrorBoundary>
    );
  }

  if (roomOpen) {
    return (
      <ErrorBoundary level="top">
        <AppShell readingFontSize={readingFontSize}>
          <CampaignTitleBar
            language={uiLanguage}
            campaignTitle={tCampaigns('coop')}
            onLanguageChange={setUiLanguage}
            onOpenSettings={() => {}}
            hideSettings
          />
          <RoomExperience
            campaigns={campaignsFeature.campaigns}
            campaignsLoading={campaignsFeature.loading}
            campaignsError={Boolean(campaignsFeature.errorCode)}
            onRefreshCampaigns={() => void campaignsFeature.refresh()}
            onBack={() =>
              exitRoomToLocalCampaign(persistedCampaignId, campaignsFeature.activate, () => {
                setRoomOpen(false);
                useStore.getState().onboarding.chooseEntry('adventure');
              })
            }
          />
        </AppShell>
      </ErrorBoundary>
    );
  }

  if (!onboardingCompleted) {
    return (
      <ErrorBoundary level="top">
        <AppShell readingFontSize={readingFontSize}>
          <CampaignTitleBar
            language={uiLanguage}
            onLanguageChange={setUiLanguage}
            onOpenSettings={() => setSettingsOpen(true)}
          />
          <main className="dm-entry-surface dm-surface-oak" />
          <Onboarding
            hideLanguage
            onJoinRoom={() => setRoomOpen(true)}
            onComplete={() => setCampaignCreateOpen(true)}
          />
          <AppOverlays>
            <SettingsModal
              open={settingsOpen}
              onClose={() => setSettingsOpen(false)}
              initialTab={settingsInitialTab}
            />
          </AppOverlays>
        </AppShell>
      </ErrorBoundary>
    );
  }

  if (campaignRoute.kind !== 'game-table') {
    return (
      <ErrorBoundary level="top">
        <ProviderMigrationBanner />
        <AppShell readingFontSize={readingFontSize}>
          <CampaignTitleBar
            language={uiLanguage}
            {...(campaignsFeature.active?.title
              ? { campaignTitle: campaignsFeature.active.title }
              : {})}
            onLanguageChange={setUiLanguage}
            onOpenSettings={() => setSettingsOpen(true)}
            onOpenRoom={enterRooms}
          />
          {campaignRoute.kind === 'campaign-create' ? (
            <CampaignCreate
              busy={campaignsFeature.loading}
              onBack={() => setCampaignCreateOpen(false)}
              onCreate={async (input) => {
                const campaign = await campaignsFeature.create(input);
                if (campaign) {
                  useStore.getState().charCreation.resetDraft();
                  setCampaignCreateOpen(false);
                } else throw new Error('campaign_preparation_failed');
              }}
            />
          ) : campaignRoute.kind === 'character-create' ? (
            <CampaignHeroChoice
              campaignId={campaignRoute.campaignId}
              pending={campaignsFeature.loading}
              onCommit={campaignsFeature.commitCharacter}
              onCustom={() => setCustomHeroCampaign(campaignRoute.campaignId)}
              onBack={() => void campaignsFeature.clear(true)}
            />
          ) : campaignRoute.kind === 'world-preparation' ? (
            <main className="dm-entry-surface">
              <p>{tCampaigns('world_not_ready')}</p>
              <button type="button" onClick={() => setCampaignCreateOpen(true)}>
                {tCampaigns('source_bundled')}
              </button>
              <button type="button" onClick={() => void campaignsFeature.clear(true)}>
                {tCampaigns('back_library')}
              </button>
            </main>
          ) : (
            <CampaignHome
              errorCode={campaignsFeature.errorCode}
              onRetry={() => {
                void (persistedCampaignId
                  ? campaignsFeature.activate(persistedCampaignId)
                  : campaignsFeature.refresh());
              }}
              campaigns={campaignsFeature.campaigns}
              loading={campaignsFeature.loading}
              onContinue={(campaignId) => {
                void campaignsFeature.activate(campaignId);
              }}
              onNew={() => setCampaignCreateOpen(true)}
            />
          )}
          <AppOverlays>
            <SettingsModal
              open={settingsOpen}
              onClose={() => setSettingsOpen(false)}
              initialTab={settingsInitialTab}
              {...(campaignRoute.kind === 'character-create'
                ? {
                    onRequestCharacterRecreate: () => {
                      setSettingsOpen(false);
                      setCustomHeroCampaign(campaignRoute.campaignId);
                    },
                  }
                : {})}
            />
            {campaignRoute.kind === 'character-create' &&
              customHeroCampaign === campaignRoute.campaignId && (
                <CharacterWizard
                  mode="initial"
                  allowCancel
                  onClose={() => setCustomHeroCampaign(null)}
                  onCommitCharacter={campaignsFeature.commitCharacter}
                  onOpenImageSettings={handleOpenImageSettings}
                  hidden={settingsOpen}
                />
              )}
          </AppOverlays>
        </AppShell>
      </ErrorBoundary>
    );
  }

  return (
    <ErrorBoundary level="top">
      <ProviderMigrationBanner />
      <AppShell readingFontSize={readingFontSize}>
        <GameTableExperience
          ports={{
            send: (text) => void sendAgentTurn(text),
            cancel: agentTurn.cancel,
            retry: () => void recovery.retry(),
            resume: () => void recovery.resume(),
            openHero: () => setCharacterSheetOpen(true),
            openJournal,
            openNpcs,
            configure: () => {
              setSettingsOpen(true);
            },
          }}
          model={soloTable({
            campaign: campaignsFeature.active as NonNullable<typeof campaignsFeature.active>,
            timeline: sessionGeneration,
            character: useStore.getState().pc,
            title: currentScene?.name ?? null,
            map: combatAuthored?.map ?? useStore.getState().session.adventure?.map ?? null,
            art: useStore.getState().session.mapMediaRef ?? null,
            tokens: combatTokens,
            activeFight: combatActive,
            encounterId: combatEncounterId,
            messages: useStore.getState().chat.messages,
            outcomes: toolEntries,
            configured: activeProviderConfig !== null,
            runtimeReady:
              activeProvider === 'local-mistralrs' ? localChatReady : activeProviderConfig !== null,
            busy: agentStreaming,
            recovery:
              recovery.status === 'loading'
                ? 'checking'
                : recovery.status === 'error'
                  ? 'unavailable'
                  : recovery.blocked
                    ? 'pending'
                    : 'clear',
          })}
        >
          <header className="dm-titlebar" data-tauri-drag-region>
            <div className="dm-titlebar-left">
              <div className="dm-app-mark">
                <div className="dm-app-mark-glyph" data-testid="brand-crest">
                  <Icons.Crest size={18} strokeWidth={1.35} />
                </div>
                <span className="dm-app-mark-name">DUNGEON MASTER AI</span>
              </div>
              <div className="dm-titlebar-divider" />
              <div className="dm-titlebar-campaign">
                <Icons.Book size={12} />
                <span>{campaignsFeature.active?.title ?? t('campaign_default')}</span>
              </div>
            </div>

            <div className="dm-titlebar-center">
              <ScenePill scene={currentScene} />
            </div>

            <div className="dm-titlebar-right">
              <button type="button" className="dm-btn-tb" onClick={enterRooms}>
                {tCampaigns('coop')}
              </button>
              {adventureFeature.projection && (
                <button
                  type="button"
                  className="dm-btn-tb"
                  onClick={() => setAdventureOpen(true)}
                  aria-label={tAdventure('open')}
                  title={tAdventure('open')}
                >
                  <Icons.BookOpen size={13} />
                  <span>{tAdventure('open')}</span>
                </button>
              )}
              <button
                type="button"
                className="dm-btn-tb"
                onClick={() => void campaignsFeature.clear()}
                aria-label={tCampaigns('back_library')}
                title={tCampaigns('back_library')}
              >
                <Icons.Book size={13} />
                <span>{tCampaigns('back_library')}</span>
              </button>
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
                  data-testid="window-minimize"
                  onClick={() => void tauriWindowAction('minimize')}
                  aria-label={t('minimize')}
                  title={t('minimize')}
                >
                  <Icons.Minimize size={10} />
                </button>
                <button
                  type="button"
                  className="dm-window-ctrl"
                  data-testid="window-maximize"
                  onClick={() => void tauriWindowAction('toggleMaximize')}
                  aria-label={t('maximize')}
                  title={t('maximize')}
                >
                  <Icons.Square size={10} />
                </button>
                <button
                  type="button"
                  className="dm-window-ctrl dm-window-close"
                  data-testid="window-close"
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
            <GameTable
              activeContext={selectedTableContext}
              onContextChange={selectTableContext}
              contextPanels={[
                {
                  id: 'adventure',
                  label: t('scene'),
                  content: adventureFeature.projection ? (
                    <AdventureContent
                      projection={adventureFeature.projection}
                      pending={adventureFeature.pending || recovery.blocked}
                      error={adventureFeature.error}
                      retryRequired={adventureFeature.retryRequired}
                      onAction={(action) => void adventureFeature.perform(action)}
                      onRetry={() => void adventureFeature.retry()}
                    />
                  ) : (
                    <p>{currentScene?.name}</p>
                  ),
                },
                {
                  id: 'hero',
                  label: t('hero'),
                  content: (
                    <>
                      <HeroContext character={useStore.getState().pc} />
                      <Button onClick={() => setCharacterSheetOpen(true)}>
                        {tCharacter('context_full_sheet')}
                      </Button>
                    </>
                  ),
                },
                {
                  id: 'journal',
                  label: t('journal'),
                  content: (
                    <JournalViewer
                      embedded
                      entries={journalEntries}
                      onClose={() => selectTableContext(null)}
                    />
                  ),
                },
                {
                  id: 'npcs',
                  label: t('npcs'),
                  content: (
                    <NpcMemoryGrid
                      embedded
                      npcs={npcList}
                      onClose={() => selectTableContext(null)}
                    />
                  ),
                },
              ]}
              renderPortrait={(actor) => (
                <ActorPortrait
                  actor={{
                    characterId: actor.characterId,
                    heroClass: actor.classId,
                    portraitAssetId: actor.portraitAssetId,
                    portraitMediaRef: actor.portraitMediaRef,
                    mediaScope: persistedCampaignId
                      ? { kind: 'campaign', campaign_id: persistedCampaignId }
                      : null,
                  }}
                />
              )}
              scene={
                <main className="dm-vtt-panel">
                  <VttCanvas
                    selectedTokenId={combatSelection.selectedId}
                    onSelectToken={combatSelection.select}
                    onMoveToken={
                      recovery.blocked
                        ? undefined
                        : (id, x, y) => void combatCommands.move(id, x, y)
                    }
                  />
                  {combatActive && (
                    <InitiativeTracker
                      compact
                      tokens={combatTokens}
                      order={combatOrder}
                      round={combatRound}
                      activeTokenId={currentTurnId}
                      selectedTokenId={combatSelection.selectedId}
                      onSelect={combatSelection.select}
                    />
                  )}
                  {showActionBar && !recovery.blocked && (
                    <ActionBar
                      slots={ownedCombatToken?.casting?.slots}
                      pending={combatCommands.pending}
                      onAttack={
                        ownedCombatToken?.loadout
                          ? () => setWeaponAttackScope(currentAttackScope)
                          : undefined
                      }
                      castAvailable={
                        ownedCombatToken?.casting ? canCast(ownedCombatToken) : undefined
                      }
                      onCast={
                        ownedCombatToken?.casting
                          ? () => setSpellCastScope(currentAttackScope)
                          : () => void combatCommands.cast()
                      }
                      onEndTurn={() => void combatCommands.endTurn()}
                      onRetreat={combatAuthored ? () => void combatCommands.retreat() : undefined}
                      retreatAvailable={Boolean(
                        combatAuthored &&
                          ownedCombatToken &&
                          combatAuthored.map.exits.some(
                            (exit) =>
                              exit.x === ownedCombatToken.x && exit.y === ownedCombatToken.y,
                          ),
                      )}
                      onIntent={(text) => {
                        void sendAgentTurn(text);
                      }}
                    />
                  )}
                  {weaponAttackScope === currentAttackScope &&
                    showActionBar &&
                    ownedCombatToken?.loadout && (
                      <WeaponAttackDialog
                        key={currentAttackScope}
                        actor={ownedCombatToken}
                        initialTargetId={combatSelection.selectedId}
                        targets={combatTokens.filter(
                          (token) =>
                            token.id !== ownedCombatToken.id && !token.characterId && token.hp > 0,
                        )}
                        onClose={() => setWeaponAttackScope(null)}
                        onAttack={combatCommands.weaponAttack}
                      />
                    )}
                  {spellCastScope === currentAttackScope &&
                    showActionBar &&
                    ownedCombatToken?.casting && (
                      <SpellCastDialog
                        key={currentAttackScope}
                        actor={ownedCombatToken}
                        initialTargetId={combatSelection.selectedId}
                        targets={combatTokens}
                        onClose={() => setSpellCastScope(null)}
                        onCast={combatCommands.spell}
                      />
                    )}
                  <CharFab
                    onOpen={() => selectTableContext('hero')}
                    onOpenWizard={() => setWizardReopen(true)}
                  />
                </main>
              }
              chat={
                <ChatPanel
                  needsSetup={needsModelSetup}
                  onRequireSetup={() => {
                    if (activeProvider === 'local-mistralrs') {
                      setSettingsInitialTab('local-llm');
                      setSettingsOpen(true);
                    } else handleOpenChatSettings();
                  }}
                  onInspect={() => setInspectorOpen(true)}
                  controller={agentTurn}
                  recoveryNotice={recoveryNotice}
                  recoveryBlocked={recovery.blocked && !recovery.clarifying}
                />
              }
              resizer={<ChatResizer />}
            />
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

          <AppOverlays>
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
            {savesOpen && <SavesScreen />}
            <QuickSaveToast lastQuickSaveAt={lastQuickSaveAt} />
            <SaveErrorToast />
            <ToolInspectorDrawer
              entries={toolEntries}
              isOpen={inspectorOpen}
              onClose={() => setInspectorOpen(false)}
            />
            <CharacterSheet
              open={characterSheetOpen}
              onClose={() => setCharacterSheetOpen(false)}
              onRest={
                pcCharacterId && !recovery.blocked
                  ? () => {
                      setCharacterSheetOpen(false);
                      setRestScope(currentRestScope);
                    }
                  : undefined
              }
            />
            {restScope === currentRestScope && pcCharacterId && (
              <RestDialog key={currentRestScope} onClose={() => setRestScope(null)} />
            )}
            {wizardReopen && (
              <CharacterWizard
                mode={pcHeroClass ? 'edit' : 'initial'}
                onCommitCharacter={campaignsFeature.commitCharacter}
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
          </AppOverlays>
        </GameTableExperience>
      </AppShell>
    </ErrorBoundary>
  );
}

export default App;
