import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
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
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr 480px',
        gridTemplateRows: 'auto 1fr',
        height: '100vh',
        background: 'var(--color-bg-base)',
      }}
    >
      <header
        style={{
          gridColumn: '1 / -1',
          padding: 'var(--space-3) var(--space-4)',
          borderBottom: '1px solid var(--color-border-strong)',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <h1 style={{ margin: 0, fontSize: 'var(--text-lg)' }}>{t('app_title')}</h1>
        <button type="button" onClick={() => setSettingsOpen(true)}>
          {t('settings')}
        </button>
      </header>

      <main style={{ overflow: 'hidden' }}>
        <VttCanvas />
      </main>

      <aside
        style={{
          borderLeft: '1px solid var(--color-border-strong)',
          overflow: 'hidden',
        }}
      >
        <ChatPanel />
      </aside>

      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}

export default App;
