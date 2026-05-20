import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { postSettingsV2 } from '../api/settings';
import { useClosingAnimation } from '../hooks/useClosingAnimation';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import { SettingsForm, type SettingsSubmission, type Tab as SettingsTab } from './SettingsForm';
import styles from './SettingsModal.module.css';

/**
 * Mirrors the shape used by `POST /local-mode/config` on the backend so the
 * Settings Save can persist the selected Qwen variant + VRAM strategy when
 * the user picks the local-mistralrs provider. Fire-and-forget; errors here
 * are non-fatal because /settings already configured the provider.
 */
function persistLocalModeConfig(): void {
  const lm = useStore.getState().localMode;
  void fetch('/local-mode/config', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      selected_llm: lm.selectedLlm,
      vram_strategy: lm.vramStrategy,
    }),
  }).catch(() => {
    // The next save retry will surface the error; the provider switch above
    // already succeeded so we do not block modal close on this.
  });
}

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

  const setActiveProvider = useStore((s) => s.settings.setActiveProvider);
  const setProviderConfig = useStore((s) => s.settings.setProviderConfig);
  const setUiLang = useStore((s) => s.settings.setUiLanguage);
  const setNarrationLang = useStore((s) => s.settings.setNarrationLanguage);
  const setSystemPrompt = useStore((s) => s.settings.setSystemPrompt);
  const setTemperature = useStore((s) => s.settings.setTemperature);
  const setReplicateApiKey = useStore((s) => s.settings.setReplicateApiKey);

  const [submitError, setSubmitError] = useState<string | null>(null);

  const { isClosing, triggerClose } = useClosingAnimation(onClose);

  // Clear stale errors when the modal closes so a re-open starts fresh.
  useEffect(() => {
    if (!open && !isClosing) setSubmitError(null);
  }, [open, isClosing]);

  const onSubmit = async (submission: SettingsSubmission) => {
    setProviderConfig(submission.provider);
    setActiveProvider(submission.provider.kind);
    setUiLang(submission.uiLanguage);
    setNarrationLang(submission.narrationLanguage);
    setSystemPrompt(submission.systemPrompt);
    setTemperature(submission.temperature);
    setReplicateApiKey(submission.replicateApiKey.length > 0 ? submission.replicateApiKey : null);

    // Read the full settings after the store updates settle in this tick.
    // Zustand updates are synchronous so .getState() now returns the merged shape.
    try {
      await postSettingsV2(useStore.getState().settings);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setSubmitError(message);
      return;
    }

    if (submission.provider.kind === 'local-mistralrs') {
      persistLocalModeConfig();
    }

    setSubmitError(null);
    triggerClose();
  };

  return (
    <Modal
      open={open || isClosing}
      onClose={triggerClose}
      closing={isClosing}
      title={t('title')}
      footer={
        <>
          <Button onClick={triggerClose}>{tCommon('cancel')}</Button>
          <Button variant="primary" type="submit" form={FORM_ID}>
            {tCommon('save')}
          </Button>
        </>
      }
    >
      {submitError && (
        <div role="alert" className={styles.errorBanner} data-testid="settings-save-error">
          {t('save_error_prefix')} {submitError}
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
