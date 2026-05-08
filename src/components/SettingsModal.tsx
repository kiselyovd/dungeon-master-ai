import { useTranslation } from 'react-i18next';
import { type AgentSettingsRequest, postAgentSettings } from '../api/agentSettings';
import { postSettings } from '../api/providers';
import { useStore } from '../state/useStore';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';
import { SettingsForm, type SettingsSubmission } from './SettingsForm';

interface Props {
  open: boolean;
  onClose: () => void;
}

const FORM_ID = 'settings-form';

export function SettingsModal({ open, onClose }: Props) {
  const { t } = useTranslation('settings');
  const { t: tCommon } = useTranslation('common');

  const setActiveProvider = useStore((s) => s.settings.setActiveProvider);
  const setProviderConfig = useStore((s) => s.settings.setProviderConfig);
  const setUiLang = useStore((s) => s.settings.setUiLanguage);
  const setNarrationLang = useStore((s) => s.settings.setNarrationLanguage);
  const setSystemPrompt = useStore((s) => s.settings.setSystemPrompt);
  const setTemperature = useStore((s) => s.settings.setTemperature);
  const setReplicateApiKey = useStore((s) => s.settings.setReplicateApiKey);

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
    // knobs in the same save. Errors are non-fatal: local persistence already
    // succeeded, so a network blip just means the sidecar keeps using the
    // previously-configured values until the next save attempt.
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
      // Surface to the chat slice so the existing error renderer picks it up.
      const { setError } = useStore.getState().chat;
      if (err instanceof Error) {
        setError({ code: 'provider_error', message: err.message });
      }
    }

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
      <SettingsForm formId={FORM_ID} onSubmit={onSubmit} />
    </Modal>
  );
}
