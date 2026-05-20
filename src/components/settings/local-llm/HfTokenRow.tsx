import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { clearToken, getTokenStatus, type TokenStatus } from '../../../api/hf';
import { HfTokenModal } from './HfTokenModal';
import styles from './HfTokenRow.module.css';

/**
 * Compact status row for the Hugging Face access token. Renders the masked
 * prefix when a token is stored, plus Add/Replace and Remove buttons. The
 * actual token entry happens inside `HfTokenModal`; this row only owns the
 * status fetch and the modal open/close state.
 */
export function HfTokenRow() {
  const { t } = useTranslation('local_llm');
  const [status, setStatus] = useState<TokenStatus>({ connected: false });
  const [modalOpen, setModalOpen] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setStatus(await getTokenStatus());
    } catch {
      setStatus({ connected: false });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className={styles.row}>
      <span>
        {t('hf_token_status')}:{' '}
        {status.connected ? (
          <strong data-testid="token-prefix">{status.prefix}</strong>
        ) : (
          <em>{t('hf_token_not_set')}</em>
        )}
      </span>
      <button type="button" onClick={() => setModalOpen(true)}>
        {status.connected ? t('hf_token_replace') : t('hf_token_add')}
      </button>
      {status.connected && (
        <button
          type="button"
          onClick={() => {
            void (async () => {
              await clearToken();
              await refresh();
            })();
          }}
        >
          {t('hf_token_remove')}
        </button>
      )}
      <HfTokenModal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        onSaved={() => {
          void refresh();
        }}
      />
    </div>
  );
}
