import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Field, Modal } from '../../ui';
import { Select } from '../../ui/Select';
import { useRest } from './useRest';
export function RestDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('character');
  const rest = useRest();
  const [kind, setKind] = useState<'short' | 'long' | 'wait'>('short');
  const [dice, setDice] = useState(0);
  const [minutes, setMinutes] = useState(960);
  const [completed, setCompleted] = useState(false);
  const status = rest.status;
  const selectedDice = Math.min(dice, status?.hit_dice_remaining ?? 0);
  const blocked =
    !status ||
    status.combat_active ||
    !status.safe ||
    status.hp <= 0 ||
    (kind === 'long' && status.long_rest_ready_in_seconds > 0);
  const confirm = async () => {
    setCompleted(false);
    const accepted = await rest.perform(
      kind === 'short'
        ? { kind, hit_dice: selectedDice }
        : kind === 'wait'
          ? { kind, minutes }
          : { kind },
    );
    if (accepted) setCompleted(true);
  };
  const hours = Math.floor((status?.elapsed_seconds ?? 0) / 3600);
  const elapsedMinutes = Math.floor(((status?.elapsed_seconds ?? 0) % 3600) / 60);
  return (
    <Modal
      open
      onClose={onClose}
      disableClose={rest.busy || rest.retryRequired}
      title={t('rest_title')}
      footer={
        <>
          <Button onClick={onClose} disabled={rest.busy || rest.retryRequired}>
            {t('rest_close')}
          </Button>
          {rest.retryRequired ? (
            <Button variant="primary" disabled={rest.busy} onClick={() => void rest.retry()}>
              {t('rest_retry')}
            </Button>
          ) : (
            <Button
              variant="primary"
              disabled={rest.loading || rest.busy || blocked}
              onClick={() => void confirm()}
            >
              {t('rest_confirm')}
            </Button>
          )}
        </>
      }
    >
      {rest.loading && <p role="status">{t('rest_loading')}</p>}
      {status && (
        <>
          <p>
            {t('rest_hp', { hp: status.hp, max: status.max_hp })} ·{' '}
            {t('rest_hit_dice', { remaining: status.hit_dice_remaining, die: status.hit_die })}
          </p>
          <p>{t('rest_elapsed', { hours, minutes: String(elapsedMinutes).padStart(2, '0') })}</p>
          {Object.entries(status.slots).map(([level, stock]) => (
            <p key={level}>
              {t('rest_slots', { level, remaining: stock.remaining, maximum: stock.maximum })}
            </p>
          ))}
          <Field label={t('rest_kind')}>
            {({ id }) => (
              <Select
                id={id}
                value={kind}
                disabled={rest.busy || rest.retryRequired}
                options={[
                  { value: 'short', label: t('rest_short') },
                  { value: 'long', label: t('rest_long') },
                  { value: 'wait', label: t('rest_wait') },
                ]}
                onChange={setKind}
              />
            )}
          </Field>
          {kind === 'short' && (
            <Field label={t('rest_dice_select')}>
              {({ id }) => (
                <Select
                  id={id}
                  value={String(selectedDice)}
                  disabled={rest.busy || rest.retryRequired}
                  options={Array.from({ length: status.hit_dice_remaining + 1 }, (_, count) => ({
                    value: String(count),
                    label: String(count),
                  }))}
                  onChange={(value) => setDice(Number(value))}
                />
              )}
            </Field>
          )}
          {kind === 'wait' && (
            <Field label={t('rest_wait_select')}>
              {({ id }) => (
                <Select
                  id={id}
                  value={String(minutes)}
                  disabled={rest.busy || rest.retryRequired}
                  options={[60, 240, 960, 1440].map((minutes) => ({
                    value: String(minutes),
                    label: t('rest_wait_hours', { hours: minutes / 60 }),
                  }))}
                  onChange={(value) => setMinutes(Number(value))}
                />
              )}
            </Field>
          )}
          {status.long_rest_ready_in_seconds > 0 && (
            <p>
              {t('rest_cooldown', { hours: Math.ceil(status.long_rest_ready_in_seconds / 3600) })}
            </p>
          )}
          {!status.safe && <p role="status">{t('rest_unsafe')}</p>}
          {status.combat_active && <p role="status">{t('rest_combat')}</p>}
          {status.hp <= 0 && <p role="status">{t('rest_downed')}</p>}
        </>
      )}
      {completed && <p role="status">{t('rest_completed')}</p>}
      {rest.retryRequired && <p role="status">{t('rest_pending')}</p>}
      {rest.error && (
        <p role="alert">
          {t(`rest_errors.${rest.error}`, { defaultValue: t('rest_errors.generic') })}
        </p>
      )}
    </Modal>
  );
}
