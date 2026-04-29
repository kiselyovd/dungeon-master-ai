import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import styles from './App.module.css';
import { initBackendListener } from './api/client';
import { ChatPanel } from './components/ChatPanel';
import { SettingsModal } from './components/SettingsModal';
import { VttCanvas } from './components/VttCanvas';
import i18n from './i18n';
import { useStore } from './state/useStore';

function App() {
  const { t } = useTranslation('common');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const uiLanguage = useStore((s) => s.settings.uiLanguage);

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
    </div>
  );
}

export default App;
