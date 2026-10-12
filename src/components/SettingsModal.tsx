import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettings } from '../features/settings/useSettings';
import { useClosingAnimation } from '../hooks/useClosingAnimation';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import { SettingsForm, type SettingsSubmission, type Tab as SettingsTab } from './SettingsForm';
import styles from './SettingsModal.module.css';

interface Props {
  open: boolean;
  onClose: () => void;
  /** Optional callback invoked when the user wants to re-create their character. */
  onRequestCharacterRecreate?: () => void;
  /** Optional tab to focus on open. Forwarded to SettingsForm.initialTab. */
  initialTab?: SettingsTab;
}

const FORM_ID = 'settings-form';

export function SettingsModal({ open, onClose, onRequestCharacterRecreate, initialTab }: Props) {
  const { t } = useTranslation('settings');
  const { t: tCommon } = useTranslation('common');

  const { save, busy } = useSettings();

  const [submitError, setSubmitError] = useState<string | null>(null);

  const { isClosing, triggerClose } = useClosingAnimation(onClose);

  // Clear stale errors when the modal closes so a re-open starts fresh.
  useEffect(() => {
    if (!open && !isClosing) setSubmitError(null);
  }, [open, isClosing]);

  const onSubmit = async (submission: SettingsSubmission) => {
    setSubmitError(null);
    try {
      if (!(await save(submission))) return;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setSubmitError(
        message === 'backend_configuration_changed' || message === 'provider_configuration_timeout'
          ? t(message)
          : message,
      );
      return;
    }

    setSubmitError(null);
    triggerClose();
  };

  return (
    <Modal
      open={open || isClosing}
      onClose={() => {
        if (!busy) triggerClose();
      }}
      closing={isClosing}
      title={t('title')}
      footer={
        <>
          <Button onClick={triggerClose} disabled={busy}>
            {tCommon('cancel')}
          </Button>
          <Button variant="primary" type="submit" form={FORM_ID} disabled={busy}>
            {tCommon('save')}
          </Button>
        </>
      }
    >
      {submitError && (
        <div role="alert" className={styles.errorBanner} data-testid="settings-save-error">
          <span className={styles.errorBannerText}>
            {t('save_error_prefix')} {submitError}
          </span>
          <button
            type="button"
            className={styles.errorBannerDismiss}
            onClick={() => setSubmitError(null)}
            aria-label={tCommon('dismiss')}
          >
            {tCommon('dismiss')}
          </button>
        </div>
      )}
      <SettingsForm
        formId={FORM_ID}
        onSubmit={onSubmit}
        {...(onRequestCharacterRecreate ? { onRequestCharacterRecreate } : {})}
        {...(initialTab ? { initialTab } : {})}
      />
    </Modal>
  );
}
