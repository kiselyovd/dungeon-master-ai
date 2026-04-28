import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChatPanel } from './components/ChatPanel';
import { VttCanvas } from './components/VttCanvas';
import { SettingsModal } from './components/SettingsModal';
import { initBackendListener } from './api/client';
import { useStore } from './state/useStore';
import { getAnthropicApiKey, getUiLanguage, getNarrationLanguage } from './api/secrets';
import i18n from './i18n';

function App() {
  const { t } = useTranslation('common');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const setApiKey = useStore((s) => s.settings.setApiKey);
  const setUiLang = useStore((s) => s.settings.setUiLanguage);
  const setNarrationLang = useStore((s) => s.settings.setNarrationLanguage);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void initBackendListener().then((u) => {
      unlisten = u;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  useEffect(() => {
    void (async () => {
      const k = await getAnthropicApiKey();
      const ui = await getUiLanguage();
      const narr = await getNarrationLanguage();
      if (k) setApiKey(k);
      if (ui) {
        setUiLang(ui);
        await i18n.changeLanguage(ui);
      }
      if (narr) setNarrationLang(narr);
    })();
  }, [setApiKey, setUiLang, setNarrationLang]);

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
        <button onClick={() => setSettingsOpen(true)}>Settings</button>
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
