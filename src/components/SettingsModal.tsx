import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../state/useStore';
import {
  getAnthropicApiKey,
  setAnthropicApiKey,
  getUiLanguage,
  setUiLanguage as persistUiLang,
  getNarrationLanguage,
  setNarrationLanguage as persistNarrationLang,
} from '../api/secrets';
import type { Language } from '../state/settings';
import i18n from '../i18n';

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: Props) {
  const { t } = useTranslation('settings');
  const { t: tCommon } = useTranslation('common');
  const setApiKeyInStore = useStore((s) => s.settings.setApiKey);
  const setUiLangInStore = useStore((s) => s.settings.setUiLanguage);
  const setNarrationLangInStore = useStore((s) => s.settings.setNarrationLanguage);

  const [apiKey, setApiKey] = useState('');
  const [uiLang, setUiLang] = useState<Language>('en');
  const [narrationLang, setNarrationLang] = useState<Language>('en');

  useEffect(() => {
    if (!open) return;
    void (async () => {
      const k = await getAnthropicApiKey();
      const ui = await getUiLanguage();
      const narr = await getNarrationLanguage();
      setApiKey(k ?? '');
      if (ui) setUiLang(ui);
      if (narr) setNarrationLang(narr);
    })();
  }, [open]);

  if (!open) return null;

  const onSave = async () => {
    await setAnthropicApiKey(apiKey || undefined);
    await persistUiLang(uiLang);
    await persistNarrationLang(narrationLang);
    setApiKeyInStore(apiKey || undefined);
    setUiLangInStore(uiLang);
    setNarrationLangInStore(narrationLang);
    await i18n.changeLanguage(uiLang);
    onClose();
  };

  return (
    <div
      role="dialog"
      aria-modal="true"
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.5)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 100,
      }}
    >
      <div
        style={{
          background: 'var(--color-bg-raised)',
          padding: 'var(--space-6)',
          borderRadius: 'var(--radius-lg)',
          minWidth: 480,
          border: '1px solid var(--color-border-strong)',
        }}
      >
        <h2 style={{ marginTop: 0 }}>{t('title')}</h2>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--space-4)' }}>
          <label>
            <div style={{ marginBottom: 'var(--space-1)' }}>{t('api_key_label')}</div>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={t('api_key_placeholder')}
              style={{ width: '100%' }}
            />
          </label>

          <label>
            <div style={{ marginBottom: 'var(--space-1)' }}>{t('language_ui_label')}</div>
            <select
              value={uiLang}
              onChange={(e) => setUiLang(e.target.value as Language)}
              style={{ width: '100%' }}
            >
              <option value="en">{t('lang_en')}</option>
              <option value="ru">{t('lang_ru')}</option>
            </select>
          </label>

          <label>
            <div style={{ marginBottom: 'var(--space-1)' }}>{t('language_narration_label')}</div>
            <select
              value={narrationLang}
              onChange={(e) => setNarrationLang(e.target.value as Language)}
              style={{ width: '100%' }}
            >
              <option value="en">{t('lang_en')}</option>
              <option value="ru">{t('lang_ru')}</option>
            </select>
          </label>
        </div>

        <div
          style={{
            display: 'flex',
            gap: 'var(--space-2)',
            marginTop: 'var(--space-6)',
            justifyContent: 'flex-end',
          }}
        >
          <button onClick={onClose}>{tCommon('cancel')}</button>
          <button onClick={() => void onSave()} style={{ borderColor: 'var(--color-accent)' }}>
            {tCommon('save')}
          </button>
        </div>
      </div>
    </div>
  );
}
