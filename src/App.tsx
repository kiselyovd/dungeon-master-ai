import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './App.module.css';
import { initBackendListener } from './api/client';
import { ActionBar } from './components/ActionBar';
import { ChatPanel } from './components/ChatPanel';
import { InitiativeTracker } from './components/InitiativeTracker';
import { LocalModeModal } from './components/LocalModeModal';
import { SettingsModal } from './components/SettingsModal';
import { UpdateAvailableModal } from './components/UpdateAvailableModal';
import { VttCanvas } from './components/VttCanvas';
import { useUpdater } from './hooks/useUpdater';
import i18n from './i18n';
import { useStore } from './state/useStore';

function App() {
  const { t } = useTranslation('common');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [localModeOpen, setLocalModeOpen] = useState(false);
  const uiLanguage = useStore((s) => s.settings.uiLanguage);
  const combatActive = useStore((s) => s.combat.active);
  const combatTokens = useStore((s) => s.combat.tokens);
  const combatOrder = useStore((s) => s.combat.initiativeOrder);
  const combatRound = useStore((s) => s.combat.round);
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

  return (
    <div className={styles.app}>
      <header className={styles.header}>
        <h1 className={styles.title}>{t('app_title')}</h1>
        <button type="button" onClick={() => setLocalModeOpen(true)}>
          {t('local_mode')}
        </button>
        <button type="button" onClick={() => setSettingsOpen(true)}>
          {t('settings')}
        </button>
      </header>

      <main className={styles.canvas}>
        <VttCanvas />
      </main>

      <aside className={styles.aside}>
        <ChatPanel />
      </aside>

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

      <InitiativeTracker
        tokens={combatTokens}
        order={combatOrder}
        round={combatRound}
        visible={combatActive}
      />
      <ActionBar
        actionUsed={false}
        bonusUsed={false}
        reactionUsed={false}
        movementFt={30}
        speedFt={30}
        visible={combatActive}
        onEndTurn={() => {
          /* M3 wires end-turn to the LLM agent loop */
        }}
      />
    </div>
  );
}

export default App;
