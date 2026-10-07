import { type ReactNode, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { RoomAction } from '../../api/contracts/roomOperations';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import type { RoomView } from './model/RoomController';
import styles from './Room.module.css';

interface Props {
  battle?: ReactNode;
  view: RoomView;
  onLeave: () => void;
  onReconnect: () => void;
  onRetry: () => void;
  onCancel: () => void;
  onHero: () => void;
  onSend: (text: string) => void;
  onAction: (action: RoomAction) => void;
}
export function RoomPlay({
  view,
  battle,
  onLeave,
  onReconnect,
  onRetry,
  onCancel,
  onHero,
  onSend,
  onAction,
}: Props) {
  const { t } = useTranslation('rooms');
  const { t: ta } = useTranslation('adventure');
  const { t: tc } = useTranslation('character');
  const [text, setText] = useState('');
  const { state, connection } = view;
  const blocked =
    view.status !== 'connected' || view.busy || view.pending !== null || state?.agent_busy === true;
  return (
    <main className={styles.root}>
      <div className={styles.content}>
        <header>
          <h1>{state?.title ?? t('title')}</h1>
          <p>{t('human')}</p>
          <Button onClick={onLeave} disabled={view.busy}>
            {connection?.kind === 'host' ? t('close_room') : t('leave')}
          </Button>
        </header>
        {view.status === 'disconnected' && (
          <section role="alert">
            <p>{t('offline')}</p>
            <Button onClick={onReconnect}>{t('reconnect')}</Button>
          </section>
        )}
        {view.errorCode && <p role="alert">{t('error')}</p>}
        {view.pending && (
          <Button disabled={view.busy || view.status !== 'connected'} onClick={onRetry}>
            {t('retry')}
          </Button>
        )}
        {!state ? (
          <p role="status">{t('waiting')}</p>
        ) : (
          <>
            {battle}
            <section className={styles.card}>
              <h2>{t('party')}</h2>
              <div className={styles.actions}>
                {state.actors.map((actor) => (
                  <span key={actor.character_id}>
                    {actor.name} · {t('hp')} {actor.hp}/{actor.max_hp} · {t('ac')} {actor.ac}
                  </span>
                ))}
              </div>
              {connection?.kind === 'host' && (
                <>
                  <p className={styles.meta}>
                    {t('room_id')}: {connection.room.room_id}
                  </p>
                  <p className={styles.meta}>
                    {t('address')}: {connection.address}
                  </p>
                  <Field label={t('invitation')}>
                    {(props) => (
                      <input
                        {...props}
                        className={styles.input}
                        readOnly
                        value={connection.invitation}
                        data-room-invitation
                      />
                    )}
                  </Field>
                </>
              )}
            </section>
            <div className={styles.columns}>
              <section className={styles.card}>
                <h2>{t('own')}</h2>
                {state.own_character ? (
                  <p>{String(state.own_character.name ?? '')}</p>
                ) : state.own_proposal ? (
                  <p>{t('approval')}</p>
                ) : (
                  <Button disabled={blocked} onClick={onHero}>
                    {t('hero_create')}
                  </Button>
                )}
                {state.pending_proposals && state.pending_proposals.length > 0 && (
                  <>
                    <h2>{t('proposals')}</h2>
                    {state.pending_proposals.map((proposal) => (
                      <div key={proposal.proposal_id}>
                        <p>
                          {proposal.actor.name} ·{' '}
                          {tc(
                            `catalog.classes.${proposal.actor.class_id}` as 'catalog.classes.fighter',
                          )}
                        </p>
                        <Button
                          disabled={blocked}
                          onClick={() =>
                            onAction({
                              kind: 'approve_character',
                              proposal_id: proposal.proposal_id,
                              expected_revision: state.revision,
                              causation_id: crypto.randomUUID(),
                            })
                          }
                        >
                          {t('approve')}
                        </Button>
                      </div>
                    ))}
                  </>
                )}
                {state.adventure && (
                  <>
                    <h2>{t('actions')}</h2>
                    <p>{ta(`locations.${state.adventure.state.location}`)}</p>
                    <div className={styles.actions}>
                      {state.adventure.available_actions.map((action) => (
                        <Button
                          key={JSON.stringify(action)}
                          disabled={blocked}
                          onClick={() =>
                            onAction({
                              kind: 'adventure',
                              request_id: crypto.randomUUID(),
                              expected_revision: state.revision,
                              action,
                            })
                          }
                        >
                          {action.kind === 'move'
                            ? ta('actions.move', { location: ta(`locations.${action.location}`) })
                            : ta(`actions.${action.kind}`)}
                        </Button>
                      ))}
                    </div>
                  </>
                )}
                <div className={styles.actions}>
                  <Button
                    disabled={blocked || !state.own_character}
                    onClick={() =>
                      onAction({
                        kind: 'rest',
                        request_id: crypto.randomUUID(),
                        expected_revision: state.revision,
                        intent: { kind: 'short', hit_dice: 0 },
                      })
                    }
                  >
                    {t('short_rest')}
                  </Button>
                  {connection?.kind === 'host' && (
                    <Button
                      disabled={blocked}
                      onClick={() =>
                        onAction({
                          kind: 'rest',
                          request_id: crypto.randomUUID(),
                          expected_revision: state.revision,
                          intent: { kind: 'long' },
                        })
                      }
                    >
                      {t('long_rest')}
                    </Button>
                  )}
                </div>
              </section>
              <section className={styles.card}>
                <h2>{t('dm')}</h2>
                <div className={styles.messages}>
                  {state.messages.map((message) => (
                    <article key={message.id} className={styles.message}>
                      {message.text}
                    </article>
                  ))}
                  {view.dmText && (
                    <article className={styles.message} aria-live="polite">
                      {view.dmText}
                    </article>
                  )}
                </div>
                {view.busy && (
                  <>
                    <p role="status">{t('busy')}</p>
                    <Button onClick={onCancel}>{t('cancel')}</Button>
                  </>
                )}
                <form
                  onSubmit={(event) => {
                    event.preventDefault();
                    if (text.trim()) {
                      onSend(text);
                      setText('');
                    }
                  }}
                >
                  <Field label={t('message')}>
                    {(props) => (
                      <textarea
                        {...props}
                        className={styles.input}
                        value={text}
                        onChange={(event) => setText(event.target.value)}
                        disabled={blocked || !state.own_character}
                        required
                      />
                    )}
                  </Field>
                  <Button type="submit" disabled={blocked || !state.own_character || !text.trim()}>
                    {t('send')}
                  </Button>
                </form>
              </section>
            </div>
          </>
        )}
      </div>
    </main>
  );
}
