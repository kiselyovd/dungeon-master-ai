import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  type AuxiliaryKind,
  auxiliaryWindowsAvailable,
  openAuxiliaryWindow,
} from '../api/auxiliaryWindows';
import { Button } from '../ui/Button';

export function AuxiliaryButton({
  kind,
  disabled = false,
}: {
  kind: AuxiliaryKind;
  disabled?: boolean;
}) {
  const { t } = useTranslation('common');
  const [failure, setFailure] = useState(false);
  if (!auxiliaryWindowsAvailable()) return null;
  return (
    <>
      <Button
        disabled={disabled}
        onClick={() => {
          setFailure(false);
          void openAuxiliaryWindow(kind).catch(() => setFailure(true));
        }}
      >
        {t('aux_open')}
      </Button>
      {failure && <p role="alert">{t('context_unavailable')}</p>}
    </>
  );
}
