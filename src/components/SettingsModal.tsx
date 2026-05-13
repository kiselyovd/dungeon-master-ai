import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { type AgentSettingsRequest, postAgentSettings } from '../api/agentSettings';
import { postSettings } from '../api/providers';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import { SettingsForm, type SettingsSubmission } from './SettingsForm';
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
}

const FORM_ID = 'settings-form';

export function SettingsModal({ open, onClose, onRequestCharacterRecreate }: Props) {
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

  // Clear stale errors when the modal closes so a re-open starts fresh.
  useEffect(() => {
    if (!open) setSubmitError(null);
  }, [open]);

  const onSubmit = async (submission: SettingsSubmission) => {
    // Mutating the store fires the persist middleware, which writes through
    // to secrets.json + settings.json. The App-level effect picks up the new
    // uiLanguage and tells i18n to switch.
    setProviderConfig(submission.provider);
    setActiveProvider(submission.provider.kind);
    setUiLang(submission.uiLanguage);
    setNarrationLang(submission.narrationLanguage);
    setSystemPrompt(submission.systemPrompt);
    setTemperature(submission.temperature);
    setReplicateApiKey(submission.replicateApiKey.length > 0 ? submission.replicateApiKey : null);

    // Tell the backend to swap providers atomically and push the agent-loop
    // knobs in the same save. The chat panel is occluded by this modal, so a
    // network failure has to surface inline here - otherwise the user closes
    // the modal believing the save succeeded and only sees the error after.
    try {
      await postSettings(submission.provider);
      const agentReq: AgentSettingsRequest = {
        temperature: submission.temperature,
      };
      if (submission.systemPrompt) agentReq.system_prompt = submission.systemPrompt;
      if (submission.replicateApiKey.length > 0)
        agentReq.replicate_api_key = submission.replicateApiKey;
      await postAgentSettings(agentReq);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setSubmitError(message);
      return;
    }

    // For local-mistralrs, also persist the Qwen variant + VRAM strategy so
    // the runtime/start route picks them up. Fire-and-forget - /settings has
    // already pinned the provider so the chat path will work either way.
    if (submission.provider.kind === 'local-mistralrs') {
      persistLocalModeConfig();
    }

    setSubmitError(null);
    onClose();
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={t('title')}
      footer={
        <>
          <Button onClick={onClose}>{tCommon('cancel')}</Button>
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
      />
    </Modal>
  );
}
