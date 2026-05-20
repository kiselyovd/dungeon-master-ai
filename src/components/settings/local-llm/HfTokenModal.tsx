import { useEffect, useId, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { setToken } from '../../../api/hf';
import { Button } from '../../../ui/Button';
import { Modal } from '../../../ui/Modal';
import styles from './HfTokenModal.module.css';

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
 *
 * Focus-trap, ESC-to-close, and backdrop-click-to-close are all handled by
 * the shared Modal primitive.
 */
export function HfTokenModal({ open, onClose, onSaved }: HfTokenModalProps) {
  const { t } = useTranslation('local_llm');
  const inputId = useId();
  const errorId = `${inputId}-error`;
  const [value, setValue] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setValue('');
      setErr(null);
    }
  }, [open]);

  async function save() {
    setBusy(true);
    setErr(null);
    try {
      await setToken(value);
      onSaved();
      onClose();
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  const canSave = value.trim().length > 0 && !busy;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t('hf_token_title')}
      footer={
        <>
          <Button onClick={onClose} disabled={busy}>
            {t('cancel')}
          </Button>
          <Button
            variant="primary"
            disabled={!canSave}
            onClick={() => {
              void save();
            }}
          >
            {t('save')}
          </Button>
        </>
      }
    >
      <div className={styles.fieldRow}>
        <label htmlFor={inputId} className={styles.label}>
          {t('hf_token_label')}
        </label>
        <input
          id={inputId}
          type="password"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          className={styles.input}
          aria-invalid={err != null}
          aria-describedby={err != null ? errorId : undefined}
          // biome-ignore lint/a11y/noAutofocus: token input should receive focus immediately when the modal opens
          autoFocus
          onKeyDown={(e) => {
            if (e.key === 'Enter' && canSave) {
              void save();
            }
          }}
        />
        {err && (
          <p id={errorId} role="alert" className={styles.error}>
            {err}
          </p>
        )}
      </div>
    </Modal>
  );
}
