import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { setToken } from '../../../api/hf';

export interface HfTokenModalProps {
  open: boolean;
  onClose: () => void;
  onSaved: () => void;
}

/**
 * Password-input modal for entering a Hugging Face access token. Used by
 * `HfTokenRow` for both the "Add token" and "Replace" flows. On save the
 * token is POSTed to `/hf/token` via `api/hf.setToken`, then `onSaved` fires
 * so the parent can refresh status, and `onClose` runs to dismiss the modal.
 */
export function HfTokenModal({ open, onClose, onSaved }: HfTokenModalProps) {
  const { t } = useTranslation('local_llm');
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  if (!open) return null;

  async function save() {
    setBusy(true);
    setErr(null);
    try {
      await setToken(value);
      onSaved();
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      style={{
        position: 'fixed',
        inset: 0,
        background: 'rgba(0,0,0,0.6)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
    >
      <div style={{ background: '#1a1611', padding: 24, borderRadius: 8, minWidth: 360 }}>
        <h3>{t('hf_token_title')}</h3>
        <label htmlFor="hf-token-input" style={{ display: 'block', marginBottom: 8 }}>
          {t('hf_token_label')}
        </label>
        <input
          id="hf-token-input"
          type="password"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          style={{ width: '100%', padding: 8 }}
        />
        {err && <p style={{ color: 'crimson' }}>{err}</p>}
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, marginTop: 16 }}>
          <button type="button" onClick={onClose} disabled={busy}>
            {t('cancel')}
          </button>
          <button
            type="button"
            onClick={() => {
              void save();
            }}
            disabled={busy || !value.trim()}
          >
            {t('save')}
          </button>
        </div>
      </div>
    </div>
  );
}
