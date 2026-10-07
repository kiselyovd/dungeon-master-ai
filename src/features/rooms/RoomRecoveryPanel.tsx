import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { RoomState } from '../../api/contracts/room';
import type { RoomAction } from '../../api/contracts/roomOperations';
import { Button } from '../../ui/Button';
import { Modal } from '../../ui/Modal';
import styles from './Room.module.css';

interface Props {
  state: RoomState;
  disabled: boolean;
  onAction: (action: RoomAction) => void;
}
export function RoomRecoveryPanel({ state, disabled, onAction }: Props) {
  const { t } = useTranslation('rooms');
  const [waiting, setWaiting] = useState<{ id: string; revision: number } | null>(null);
  const wounded = state.recovery.filter((hero) => hero.status !== 'healthy');
  const chosen = state.recovery.find((hero) => hero.character_id === waiting?.id);
  const wait = chosen?.actions.find((action) => action.kind === 'wait');
  useEffect(() => {
    if (waiting && (!wait || waiting.revision !== state.revision)) setWaiting(null);
  }, [waiting, wait, state.revision]);
  if (wounded.length === 0) return null;
  function perform(intent: Extract<RoomAction, { kind: 'recovery' }>['intent']) {
    if (disabled) return;
    onAction({
      kind: 'recovery',
      request_id: crypto.randomUUID(),
      expected_revision: state.revision,
      intent,
    });
  }
  return (
    <section className={styles.card}>
      <h2>{t('recovery_title')}</h2>
      <p>{t('recovery_hint')}</p>
      {wounded.map((hero) => (
        <div key={hero.character_id}>
          <p>
            {state.actors.find((actor) => actor.character_id === hero.character_id)?.name} ·{' '}
            {t(`recovery_status.${hero.status}`)}
          </p>
          {hero.status === 'dying' && (
            <p>{t('recovery_saves', { successes: hero.successes, failures: hero.failures })}</p>
          )}
          {hero.ready_at_seconds !== null && (
            <p>
              {t('recovery_remaining', {
                minutes: Math.max(
                  0,
                  Math.ceil((hero.ready_at_seconds - state.clock.elapsed_seconds) / 60),
                ),
              })}
            </p>
          )}
          <div className={styles.actions}>
            {hero.actions.map((intent) => (
              <Button
                key={intent.kind}
                disabled={disabled}
                onClick={() =>
                  intent.kind === 'wait'
                    ? setWaiting({ id: hero.character_id, revision: state.revision })
                    : perform(intent)
                }
              >
                {t(`recovery_action.${intent.kind}`)}
              </Button>
            ))}
          </div>
          {hero.status === 'stable' && hero.actions.length === 0 && (
            <p>{t('recovery_safe_place')}</p>
          )}
        </div>
      ))}
      <Modal
        open={Boolean(waiting && wait)}
        title={t('recovery_action.wait')}
        onClose={() => setWaiting(null)}
        footer={
          <Button
            disabled={disabled || !wait}
            onClick={() => {
              if (wait) perform(wait);
              setWaiting(null);
            }}
          >
            {t('recovery_action.wait')}
          </Button>
        }
      >
        <p>{t('recovery_wait_hint')}</p>
      </Modal>
    </section>
  );
}
