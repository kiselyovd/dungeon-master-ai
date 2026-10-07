import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useStore } from '../../state/useStore';
import { Button, Field, Modal } from '../../ui';
import { Select } from '../../ui/Select';
import { useRest } from './useRest';
import { useSoloRecovery } from './useSoloRecovery';
export function RestDialog({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation('character');
  const rest = useRest();
  const recovery = useSoloRecovery();
  const revision = useStore((s) => s.campaigns.active?.revision ?? 0);
  const streaming = useStore((s) => s.chat.isStreaming);
  const [waiting, setWaiting] = useState<number | null>(null);
  const health = recovery.status?.recovery;
  const waitingIntent = health?.actions.find((a) => a.kind === 'wait');
  const busy = rest.busy || recovery.busy;
  const pending = rest.retryRequired || recovery.retryRequired;
  const recoveryBlocked =
    busy || pending || streaming || recovery.loading || recovery.status?.revision !== revision;
  useEffect(() => {
    if (waiting !== null && (waiting !== revision || !waitingIntent)) setWaiting(null);
  }, [waiting, revision, waitingIntent]);
  async function recover(
    intent: { kind: 'death_save' } | { kind: 'wait'; target_character_id: string },
  ) {
    if (recoveryBlocked) return;
    if (await recovery.perform(intent)) await rest.reload();
  }
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
      disableClose={busy || pending}
      title={t('rest_title')}
      footer={
        <>
          <Button onClick={onClose} disabled={busy || pending}>
            {t('rest_close')}
          </Button>
          {recovery.retryRequired ? (
            <Button
              variant="primary"
              disabled={busy || streaming}
              onClick={() =>
                void recovery.retry().then((accepted) => (accepted ? rest.reload() : undefined))
              }
            >
              {t('solo_recovery.retry')}
            </Button>
          ) : rest.retryRequired ? (
            <Button
              variant="primary"
              disabled={busy || streaming}
              onClick={() => void rest.retry()}
            >
              {t('rest_retry')}
            </Button>
          ) : status && status.hp <= 0 ? null : (
            <Button
              variant="primary"
              disabled={rest.loading || busy || pending || streaming || blocked}
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
          {status.hp > 0 && (
            <>
              <Field label={t('rest_kind')}>
                {({ id }) => (
                  <Select
                    id={id}
                    value={kind}
                    disabled={busy || pending}
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
                      disabled={busy || pending}
                      options={Array.from(
                        { length: status.hit_dice_remaining + 1 },
                        (_, count) => ({
                          value: String(count),
                          label: String(count),
                        }),
                      )}
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
                      disabled={busy || pending}
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
                  {t('rest_cooldown', {
                    hours: Math.ceil(status.long_rest_ready_in_seconds / 3600),
                  })}
                </p>
              )}
            </>
          )}
          {!status.safe && <p role="status">{t('rest_unsafe')}</p>}
          {status.combat_active && <p role="status">{t('rest_combat')}</p>}
          {status.hp <= 0 && <p role="status">{t('rest_downed')}</p>}
        </>
      )}
      {health && health.status !== 'healthy' && (
        <section aria-label={t('solo_recovery.title')}>
          <h3>{t('solo_recovery.title')}</h3>
          <p>{t(`solo_recovery.status.${health.status}`)}</p>
          {health.status === 'dying' && (
            <p>
              {t('solo_recovery.saves', { successes: health.successes, failures: health.failures })}
            </p>
          )}
          {health.status === 'stable' && <p>{t('solo_recovery.stable_hint')}</p>}
          {health.ready_at_seconds !== null && (
            <p>
              {t('solo_recovery.remaining', {
                minutes: Math.max(
                  0,
                  Math.ceil(
                    (health.ready_at_seconds - (recovery.status?.elapsed_seconds ?? 0)) / 60,
                  ),
                ),
              })}
            </p>
          )}
          {health.actions.map((intent) =>
            intent.kind === 'aid' ? null : (
              <Button
                key={intent.kind}
                disabled={recoveryBlocked}
                onClick={() =>
                  intent.kind === 'wait' ? setWaiting(revision) : void recover(intent)
                }
              >
                {t(`solo_recovery.action.${intent.kind}`)}
              </Button>
            ),
          )}
          {health.status === 'stable' && !waitingIntent && <p>{t('solo_recovery.unsafe')}</p>}
        </section>
      )}
      {recovery.receipt?.death_saves.map((save, index) => (
        <p role="status" key={`${save.character_id}:${index}`}>
          {t('solo_recovery.roll', { roll: save.result.roll })}
        </p>
      ))}
      {recovery.retryRequired && <p role="status">{t('rest_pending')}</p>}
      {recovery.error && <p role="alert">{t('solo_recovery.error')}</p>}
      <Modal
        open={waiting !== null && Boolean(waitingIntent)}
        title={t('solo_recovery.action.wait')}
        onClose={() => setWaiting(null)}
        footer={
          <Button
            variant="primary"
            disabled={recoveryBlocked || waiting !== revision || !waitingIntent}
            onClick={() => {
              if (waitingIntent && waiting === revision) void recover(waitingIntent);
              setWaiting(null);
            }}
          >
            {t('solo_recovery.action.wait')}
          </Button>
        }
      >
        <p>{t('solo_recovery.wait_hint')}</p>
      </Modal>
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
