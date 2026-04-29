import { useTranslation } from 'react-i18next';
import {
  saveActiveProvider,
  saveNarrationLanguage,
  saveProviders,
  saveUiLanguage,
} from '../api/settingsStore';
import i18n from '../i18n';
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

  const onSubmit = async (submission: SettingsSubmission) => {
    setProviderConfig(submission.provider);
    setActiveProvider(submission.provider.kind);
    setUiLang(submission.uiLanguage);
    setNarrationLang(submission.narrationLanguage);

    const providers = useStore.getState().settings.providers;
    await Promise.all([
      saveProviders(providers),
      saveActiveProvider(submission.provider.kind),
      saveUiLanguage(submission.uiLanguage),
      saveNarrationLanguage(submission.narrationLanguage),
    ]);

    if (i18n.language !== submission.uiLanguage) {
      await i18n.changeLanguage(submission.uiLanguage);
    }

    // Backend hot-swap: best-effort. C4 wires the actual POST /settings call.
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
