import { type ReactNode, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import type { RoomAction } from '../../api/contracts/roomOperations';
import { MessageBubble } from '../../components/MessageBubble';
import { useConversationDraft } from '../../components/useConversationDraft';
import { useStickyScroll } from '../../hooks/useStickyScroll';
import { MAX_CONVERSATION_DRAFT_LENGTH } from '../../state/conversationDrafts';
import { Button } from '../../ui/Button';
import { Field } from '../../ui/Field';
import type { RoomView } from './model/RoomController';
import styles from './Room.module.css';
import { RoomInvitationShare } from './RoomInvitationShare';
import { RoomRecoveryPanel } from './RoomRecoveryPanel';

interface Props {
  notificationControl?: ReactNode;
  portabilityControl?: ReactNode;
  renderTable?: (slots: RoomTableSlots) => ReactNode;
  invitation?: string;
  onRefreshInvitation?: (() => void) | undefined;
  battle?: ReactNode;
  recoveryNotice?: ReactNode;
  view: RoomView;
  onLeave: () => void;
  onReconnect: () => void;
  onRetry: () => void;
  onCancel: () => void;
  onHero: () => void;
  onSend: (text: string) => boolean | undefined | Promise<boolean | undefined>;
  onAction: (action: RoomAction) => void;
}
export interface RoomTableSlots {
  scene: ReactNode;
  chat: ReactNode;
  context: ReactNode;
  roomControls: ReactNode;
  notices: ReactNode;
}
export function RoomPlay({
  notificationControl,
  portabilityControl,
  view,
  invitation,
  onRefreshInvitation,
  battle,
  recoveryNotice,
  onLeave,
  onReconnect,
  onRetry,
  onCancel,
  onHero,
  renderTable,
  onSend,
  onAction,
}: Props) {
  const { t } = useTranslation('rooms');
  const { t: ta } = useTranslation('adventure');
  const { t: tc } = useTranslation('character');
  const { t: te } = useTranslation('errors');
  const { state, connection } = view;
  const { t: tChat } = useTranslation('chat');
  const draft = useConversationDraft(
    `room:${state?.room.room_id}:${state?.room.epoch}:${connection?.member_id}`,
  );
  const text = draft.text;
  const setText = draft.setText;
  const sending = useRef(false);
  const history = useStickyScroll(100);
  // biome-ignore lint/correctness/useExhaustiveDependencies: new public output triggers sticky scrolling; scroll-away is respected
  useEffect(() => {
    history.scrollToBottom();
  }, [state?.messages.length, view.dmText, history.scrollToBottom]);
  const blocked =
    view.status !== 'connected' ||
    view.busy ||
    ((view.pending !== null || state?.agent_busy === true) &&
      !(view.pending?.kind === 'dm_turn' && view.pending.awaiting_clarification));
  const submit = async () => {
    if (blocked || !state?.own_character || !text.trim() || sending.current) return;
    sending.current = true;
    try {
      const accepted = await onSend(text);
      if (accepted !== false) setText((current) => (current === text ? '' : current));
    } catch {
      // The controller supplies the safe error projection. Keep the input.
    } finally {
      sending.current = false;
    }
  };
  const notices = (
    <>
      {' '}
      {view.status === 'disconnected' && (
        <section role="alert">
          <p>{t('offline')}</p>
          <Button onClick={onReconnect}>{t('reconnect')}</Button>
        </section>
      )}
      {view.errorCode &&
        (!renderTable ||
          !['native_host_stop_failed', 'native_host_invitation_failed'].includes(
            view.errorCode,
          )) && (
          <p role="alert">
            {view.errorCode === 'native_room_dm_failed' || view.errorCode === 'agent_turn_failed'
              ? te('agent_turn_failed')
              : view.errorCode === 'native_room_original_environment_changed'
                ? te('agent_original_environment_changed')
                : view.errorCode === 'native_room_original_environment_unavailable'
                  ? te('agent_original_environment_unavailable')
                  : t('error')}
          </p>
        )}
      {!renderTable && recoveryNotice}
      {view.pending && (view.pending.kind !== 'dm_turn' || !recoveryNotice) && (
        <Button disabled={view.busy || view.status !== 'connected'} onClick={onRetry}>
          {t('retry')}
        </Button>
      )}
    </>
  );
  const roomControls = (
    <section className={styles.card}>
      {notificationControl}
      {portabilityControl}
      <h2>{state?.title ?? t('title')}</h2>
      {(view.errorCode === 'native_host_stop_failed' ||
        view.errorCode === 'native_host_invitation_failed') && <p role="alert">{t('error')}</p>}
      {connection?.kind === 'host' && (
        <>
          <p className={styles.meta}>
            {t('room_id')}: {connection.room.room_id}
          </p>
          <p className={styles.meta}>
            {t('address')}: {connection.address}
          </p>
          <RoomInvitationShare
            address={connection.address}
            roomId={connection.room.room_id}
            invitation={invitation ?? connection.invitation}
            busy={view.busy}
          />
          {onRefreshInvitation && (
            <Button disabled={view.busy} onClick={onRefreshInvitation}>
              {t('refresh_invitation')}
            </Button>
          )}
        </>
      )}
      <Button onClick={onLeave} disabled={view.busy}>
        {connection?.kind === 'host' ? t('close_room') : t('leave')}
      </Button>
    </section>
  );
  const context = state ? (
    <>
      {' '}
      <RoomRecoveryPanel
        key={view.timelineReset}
        state={state}
        disabled={blocked}
        onAction={onAction}
      />
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
                  {tc(`catalog.classes.${proposal.actor.class_id}` as 'catalog.classes.fighter')}
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
        {state.adventure && !renderTable && (
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
    </>
  ) : (
    <p role="status">{t('waiting')}</p>
  );
  const chat = state ? (
    <section className={styles.chatCard}>
      <h2>{t('dm')}</h2>
      <div
        className={styles.messages}
        ref={history.ref}
        onScroll={history.onScroll}
        data-testid="chat-history"
      >
        {state.messages.map((message) => (
          <MessageBubble
            key={message.id}
            chatRole={message.role}
            truncated={message.truncated}
            journalEnabled={false}
          >
            {message.text}
          </MessageBubble>
        ))}
        {view.dmText && (
          <MessageBubble chatRole="assistant" streaming journalEnabled={false}>
            {view.dmText}
          </MessageBubble>
        )}
      </div>
      {view.busy && (
        <>
          <p role="status">{t('busy')}</p>
          <Button onClick={onCancel}>{t('cancel')}</Button>
        </>
      )}
      {renderTable && recoveryNotice}
      {!history.shouldScroll && (
        <Button onClick={history.jumpToBottom}>{tChat('scroll_latest')}</Button>
      )}
      {draft.failure && <p role="status">{tChat('draft_save_failed')}</p>}
      <form
        className={styles.composer}
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <Field label={t('message')}>
          {(props) => (
            <textarea
              data-command-target="chat"
              {...props}
              className={styles.input}
              value={text}
              onChange={(event) => setText(event.target.value)}
              disabled={blocked || !state.own_character}
              required
              maxLength={MAX_CONVERSATION_DRAFT_LENGTH}
              onBlur={() => void draft.flush()}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && !event.shiftKey && !event.nativeEvent.isComposing) {
                  event.preventDefault();
                  void submit();
                }
              }}
            />
          )}
        </Field>
        <Button type="submit" disabled={blocked || !state.own_character || !text.trim()}>
          {t('send')}
        </Button>
      </form>
    </section>
  ) : (
    <p role="status">{t('waiting')}</p>
  );
  const slots: RoomTableSlots = { scene: battle, chat, context, notices, roomControls };
  if (renderTable) return renderTable(slots);
  return (
    <main className={styles.root}>
      <div className={styles.content}>
        {notices}
        {battle}
        <section className={styles.card}>
          <h2>{t('party')}</h2>
          <div className={styles.actions}>
            {state?.actors.map((actor) => (
              <span key={actor.character_id}>
                {actor.name} · {t('hp')} {actor.hp}/{actor.max_hp} · {t('ac')} {actor.ac}
              </span>
            ))}
          </div>
        </section>
        {context}
        {chat}
        {roomControls}
      </div>
    </main>
  );
}
