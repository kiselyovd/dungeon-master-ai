import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { PreflightStatus } from '../lib/preflight';
import { dismissPreflight } from '../lib/preflight';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import styles from './PreflightModal.module.css';

export interface PreflightModalProps {
  status: Exclude<PreflightStatus, 'ok'>;
  onFinishSetup: () => void;
  onDismiss: () => void;
}

/**
 * Shown after onboarding completes when runPreflight detects a configuration
 * gap.
 *
 * - missing_chat: blocking - no dismiss affordance, "Finish setup" re-runs
 *   the onboarding flow.
 * - missing_image / missing_video: non-blocking - shows a "Don't ask again
 *   for 24h" checkbox, a Dismiss button, and the "Finish setup" CTA.
 */
export function PreflightModal({ status, onFinishSetup, onDismiss }: PreflightModalProps) {
  const { t } = useTranslation('common');
  const [dontAskAgain, setDontAskAgain] = useState(false);

  const isBlocking = status === 'missing_chat';

  const messageKey =
    status === 'missing_chat'
      ? 'preflight_missing_chat'
      : status === 'missing_image'
        ? 'preflight_missing_image'
        : 'preflight_missing_video';

  function handleDismiss() {
    dismissPreflight(status, dontAskAgain);
    onDismiss();
  }

  const footer = (
    <>
      {!isBlocking && (
        <Button onClick={handleDismiss} data-testid="preflight-dismiss">
          {t('preflight_dismiss')}
        </Button>
      )}
      <Button variant="primary" onClick={onFinishSetup} data-testid="preflight-finish-setup">
        {t('preflight_finish_setup')}
      </Button>
    </>
  );

  return (
    <Modal
      open
      onClose={handleDismiss}
      disableClose={isBlocking}
      title={t('preflight_title')}
      footer={footer}
    >
      <p className={styles.message}>{t(messageKey)}</p>
      {!isBlocking && (
        <label className={styles.dontAskRow}>
          <input
            type="checkbox"
            checked={dontAskAgain}
            onChange={(e) => setDontAskAgain(e.target.checked)}
            data-testid="preflight-dont-ask-again"
          />
          {t('preflight_dont_ask_again')}
        </label>
      )}
    </Modal>
  );
}
