import { useTranslation } from 'react-i18next';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';

interface DmConfirmModalProps {
  open: boolean;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  title?: string;
}

export function DmConfirmModal({ open, message, onConfirm, onCancel, title }: DmConfirmModalProps) {
  const { t } = useTranslation('common');
  return (
    <Modal
      open={open}
      onClose={onCancel}
      title={title ?? t('confirm_title')}
      footer={
        <>
          <Button onClick={onCancel}>{t('cancel')}</Button>
          {/* TODO(ux-debt): action is destructive but rendered as primary - no danger Button variant exists yet */}
          <Button variant="primary" onClick={onConfirm}>
            {t('confirm')}
          </Button>
        </>
      }
    >
      <p>{message}</p>
    </Modal>
  );
}
