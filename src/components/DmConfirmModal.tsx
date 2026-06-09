import { useTranslation } from 'react-i18next';
import { Button } from '../ui/Button';
import { Modal } from '../ui/Modal';

interface DmConfirmModalProps {
  open: boolean;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  title?: string;
  /** When true (default), the confirm button uses the danger variant. */
  destructive?: boolean;
}

export function DmConfirmModal({
  open,
  message,
  onConfirm,
  onCancel,
  title,
  destructive = true,
}: DmConfirmModalProps) {
  const { t } = useTranslation('common');
  return (
    <Modal
      open={open}
      onClose={onCancel}
      title={title ?? t('confirm_title')}
      footer={
        <>
          <Button onClick={onCancel}>{t('cancel')}</Button>
          <Button variant={destructive ? 'danger' : 'primary'} onClick={onConfirm}>
            {t('confirm')}
          </Button>
        </>
      }
    >
      <p>{message}</p>
    </Modal>
  );
}
