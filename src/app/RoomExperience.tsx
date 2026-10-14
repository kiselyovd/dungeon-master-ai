import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { fetchCampaignState } from '../api/campaigns';
import type { CampaignSummary } from '../api/contracts/campaign';
import { parseMediaReference } from '../api/contracts/mediaAsset';
import { encodeMedia, readMedia } from '../api/mediaAssets';
import {
  type GuestRecoveryChoice,
  type HostRoomRequest,
  hostRoom,
  type JoinRoomRequest,
  joinRoom,
  leaveGuestRoom,
  postponeGuestRoom,
  type RoomConnection,
  recoverGuestRoom,
  recoverHostRoom,
  resumeGuestRoom,
  savedGuestRooms,
  stopHostRoom,
} from '../api/rooms';
import { ActorPortrait } from '../components/ActorPortrait';
import { CharacterWizard } from '../components/CharacterWizard';
import { ChatResizer } from '../components/ChatResizer';
import { HeroContext } from '../components/HeroContext';
import { JournalViewer } from '../components/JournalViewer';
import { NpcMemoryGrid } from '../components/NpcMemoryGrid';
import { RecoveryPanel } from '../components/RecoveryPanel';
import { SettingsModal } from '../components/SettingsModal';
import { AdventureContent } from '../features/adventure/AdventurePanel';
import { parseCombatProjection, projectionTokens } from '../features/combat/model/combatProjection';
import { RoomCombatPanel } from '../features/combat/RoomCombatPanel';
import { CommandCenter } from '../features/commands/CommandCenter';
import { GameTable } from '../features/game-table';
import { RoomPortraitTransfer } from '../features/media/model/RoomPortraitTransfer';
import { toRecoveryView } from '../features/recovery/model/recovery';
import { RoomHome } from '../features/rooms/RoomHome';
import { RoomPlay } from '../features/rooms/RoomPlay';
import { useRoom } from '../features/rooms/useRoom';
import { useStore } from '../state/useStore';
import {
  focusTable,
  registerCommands,
  resetLayout,
  toggleFullscreen,
} from './commandRegistrations';
import { GameTableExperience } from './GameTableExperience';
import { roomContext } from './roomContext';
import { roomPorts } from './roomPorts';
import { roomTable } from './tableAdapters';

interface Props {
  campaignsLoading?: boolean | undefined;
  campaignsError?: boolean | undefined;
  onRefreshCampaigns?: (() => void) | undefined;
  campaigns: CampaignSummary[];
  onBack: () => void | Promise<void>;
}
function ConnectedRoom({
  connection,
  onBack,
}: {
  connection: RoomConnection;
  onBack: () => void | Promise<void>;
}) {
  const { view, controller } = useRoom(roomPorts, connection);
  const { t } = useTranslation('common');
  const [contextId, setContextId] = useState<string | null>(null);
  const context = useMemo(() => {
    if (!view.state) return null;
    try {
      return roomContext(view.state, connection.member_id);
    } catch {
      return null;
    }
  }, [view.state, connection.member_id]);
  const unavailableContext = view.state !== null && context === null;
  const portraitTransfer = useMemo(
    () =>
      new RoomPortraitTransfer({
        read: readMedia,
        encode: encodeMedia,
        upload: (action) => controller.perform(action),
      }),
    [controller],
  );
  async function submitCharacter(character: Record<string, unknown>) {
    const prepared = await portraitTransfer.prepare(
      character,
      connection.room.room_id,
      useStore.getState().charCreation.mediaDraftId ?? null,
      controller.getSnapshot().pending,
    );
    await controller.submitCharacter(prepared);
  }
  const [invitation, setInvitation] = useState(
    connection.kind === 'host' ? connection.invitation : '',
  );
  const [refreshingInvitation, setRefreshingInvitation] = useState(false);
  const [invitationError, setInvitationError] = useState(false);
  const [wizard, setWizard] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const [closingError, setClosingError] = useState(false);
  const confirmedDeparture = useRef<string | null>(null);
  const connectionKey = `${connection.kind}:${connection.room.room_id}:${connection.member_id}`;
  async function leave() {
    if (closing) return;
    setClosing(true);
    setClosingError(false);
    try {
      if (confirmedDeparture.current !== connectionKey) {
        if (connection.kind === 'host') await stopHostRoom(connection.room.room_id);
        else
          await leaveGuestRoom(
            connection.room.room_id,
            view.state?.room.epoch ?? connection.room.epoch ?? 1,
          );
        confirmedDeparture.current = connectionKey;
      }
      await onBack();
      controller.leave();
    } catch {
      setClosingError(true);
      setClosing(false);
    }
  }
  const tableModel = roomTable(
    { ...view, busy: view.busy || closing || refreshingInvitation },
    connection,
  );
  const combat = view.state?.combat ? parseCombatProjection(view.state.combat) : null;
  const own = combat
    ? projectionTokens(combat).find((actor) => actor.characterId === tableModel.ownActorId)
    : undefined;
  const ownTurn = Boolean(
    combat?.snapshot.active && own && own.hp > 0 && combat.snapshot.current_combatant === own.id,
  );
  const commands = registerCommands(
    {
      table: Boolean(view.state),
      guest: connection.kind === 'guest',
      room: true,
      busy:
        view.busy ||
        closing ||
        refreshingInvitation ||
        view.status !== 'connected' ||
        Boolean(view.state?.agent_busy),
      recovery: Boolean(view.pending || view.recovery),
      ownTurn,
    },
    (id) => t(`commands_${id}`),
    {
      hero: () => setContextId('hero'),
      journal: () => setContextId('journal'),
      npcs: () => setContextId('npcs'),
      room: () => focusTable('room'),
      settings: connection.kind === 'host' ? () => setSettingsOpen(true) : undefined,
      recovery: () => controller.retryPending(),
      focus_chat: () => focusTable('chat'),
      focus_map:
        tableModel.scene.kind === 'tactical' || tableModel.combat === 'active'
          ? () => focusTable('map')
          : undefined,
      reset_layout: resetLayout,
      fullscreen: toggleFullscreen,
      end_turn: () =>
        combat
          ? controller.perform({
              kind: 'combat',
              request_id: crypto.randomUUID(),
              encounter_id: combat.encounter_id,
              expected_revision: combat.revision,
              action: { kind: 'end_turn' },
            })
          : undefined,
    },
  );
  return (
    <>
      {connection.kind === 'host' && (
        <SettingsModal
          agentTurnInFlight={view.busy || Boolean(view.state?.agent_busy)}
          open={settingsOpen}
          onClose={() => setSettingsOpen(false)}
          initialTab="chat"
        />
      )}
      <GameTableExperience
        model={tableModel}
        ports={{
          send: (text) => void controller.sendDm(text),
          retry: () => void controller.retryPending(),
          cancel: () => controller.cancelDm(),
          openHero: () => {
            useStore.getState().charCreation.resetDraft();
            setWizard(true);
          },
          ...(connection.kind === 'host' ? { configure: () => setSettingsOpen(true) } : {}),
        }}
      >
        <RoomPlay
          renderTable={(slots) => (
            <GameTable
              commandLauncher={
                <CommandCenter key={`${connectionKey}:${view.timelineReset}`} commands={commands} />
              }
              {...slots}
              activeContext={contextId}
              onContextChange={setContextId}
              contextPanels={[
                {
                  id: 'adventure',
                  label: t('scene'),
                  content: view.state?.adventure ? (
                    <AdventureContent
                      projection={view.state.adventure}
                      pending={
                        view.status !== 'connected' ||
                        view.busy ||
                        Boolean(view.pending) ||
                        Boolean(view.state?.agent_busy)
                      }
                      error={Boolean(view.errorCode)}
                      retryRequired={Boolean(view.pending)}
                      onAction={(action) => {
                        if (!view.state) return;
                        void controller.perform({
                          kind: 'adventure',
                          request_id: crypto.randomUUID(),
                          expected_revision: view.state.revision,
                          action,
                        });
                      }}
                      onRetry={() => void controller.retryPending()}
                    />
                  ) : (
                    <p>{tableModel.scene.title}</p>
                  ),
                },
                {
                  id: 'hero',
                  label: t('hero'),
                  content: (
                    <>
                      {unavailableContext ? (
                        <p role="alert">{t('context_unavailable')}</p>
                      ) : (
                        <HeroContext character={context?.own ?? null} />
                      )}
                      {slots.context}
                    </>
                  ),
                },
                {
                  id: 'journal',
                  label: t('journal'),
                  content: unavailableContext ? (
                    <p role="alert">{t('context_unavailable')}</p>
                  ) : (
                    <JournalViewer
                      embedded
                      entries={context?.journal ?? []}
                      onClose={() => setContextId(null)}
                    />
                  ),
                },
                {
                  id: 'npcs',
                  label: t('npcs'),
                  content: unavailableContext ? (
                    <p role="alert">{t('context_unavailable')}</p>
                  ) : (
                    <NpcMemoryGrid
                      embedded
                      npcs={context?.npcs ?? []}
                      onClose={() => setContextId(null)}
                    />
                  ),
                },
              ]}
              resizer={<ChatResizer />}
              renderPortrait={(actor) => (
                <ActorPortrait
                  actor={{
                    characterId: actor.characterId,
                    heroClass: actor.classId,
                    portraitAssetId: actor.portraitAssetId,
                    portraitMediaRef: actor.portraitMediaRef,
                    mediaScope:
                      view.status === 'connected'
                        ? {
                            kind: 'room',
                            room_id: connection.room.room_id,
                            member_id: connection.member_id,
                            epoch: view.state?.room.epoch ?? connection.room.epoch ?? 1,
                          }
                        : null,
                  }}
                />
              )}
              scene={
                tableModel.scene.kind === 'tactical' || tableModel.combat === 'active'
                  ? slots.scene
                  : null
              }
            />
          )}
          recoveryNotice={
            view.recovery || view.recoveryStatus === 'error' || view.pending?.kind === 'dm_turn' ? (
              <RecoveryPanel
                view={
                  view.recovery
                    ? toRecoveryView(view.recovery)
                    : view.pending?.kind === 'dm_turn'
                      ? {
                          requestId: view.pending.request_id,
                          actorId: '',
                          model: null,
                          phase: view.pending.awaiting_clarification
                            ? 'needs_input'
                            : 'progress_unknown',
                          action: view.pending.awaiting_clarification ? 'clarify' : 'resume',
                          progressKnown: false,
                          totalCommands: null,
                          finishedCommands: null,
                          performedAttempts: null,
                          refusedAttempts: null,
                          clarificationId: view.pending.awaiting_clarification ?? null,
                        }
                      : null
                }
                status={view.recoveryStatus === 'idle' ? 'ready' : view.recoveryStatus}
                busy={view.busy}
                onResume={() => void controller.retryPending()}
                onRetry={() => void controller.reconnect()}
                {...(connection.kind === 'host'
                  ? { onConfigure: () => setSettingsOpen(true) }
                  : {})}
              />
            ) : null
          }
          invitation={invitation}
          onRefreshInvitation={
            connection.kind === 'host'
              ? () => {
                  if (refreshingInvitation) return;
                  setRefreshingInvitation(true);
                  setInvitationError(false);
                  void recoverHostRoom()
                    .then((current) => {
                      if (
                        current?.kind !== 'host' ||
                        current.room.room_id !== connection.room.room_id
                      )
                        throw new Error('room_scope_changed');
                      setInvitation(current.invitation);
                    })
                    .catch(() => setInvitationError(true))
                    .finally(() => setRefreshingInvitation(false));
                }
              : undefined
          }
          view={{
            ...view,
            busy: view.busy || closing || refreshingInvitation,
            errorCode: closingError
              ? 'native_host_stop_failed'
              : invitationError
                ? 'native_host_invitation_failed'
                : view.errorCode,
          }}
          battle={
            view.state && (
              <RoomCombatPanel
                mapSkinAssetId={view.state.adventure?.map_skin_asset_id ?? null}
                sceneArtwork={
                  view.state.adventure?.scene_asset_id
                    ? {
                        assetId: view.state.adventure.scene_asset_id,
                        location: view.state.adventure.state.location,
                      }
                    : null
                }
                actorVisuals={Object.fromEntries(
                  view.state.actors.map((actor) => [
                    actor.character_id,
                    {
                      characterId: actor.character_id,
                      heroClass: actor.class_id,
                      portraitAssetId: actor.portrait_asset_id ?? null,
                      portraitMediaRef: actor.portrait_media_ref ?? null,
                      mediaScope:
                        view.status === 'connected'
                          ? {
                              kind: 'room' as const,
                              room_id: connection.room.room_id,
                              member_id: connection.member_id,
                              epoch: view.state?.room.epoch ?? connection.room.epoch ?? 1,
                            }
                          : null,
                    },
                  ]),
                )}
                mapMediaRef={view.state.map_media_ref ?? null}
                raw={view.state.combat}
                timelineReset={view.timelineReset}
                map={view.state.adventure?.map ?? null}
                player={{
                  characterId:
                    view.state.actors.find((actor) => actor.member_id === connection.member_id)
                      ?.character_id ?? null,
                  heroClass:
                    typeof view.state.own_character?.heroClass === 'string'
                      ? view.state.own_character.heroClass
                      : null,
                  portraitUrl:
                    typeof view.state.own_character?.portraitUrl === 'string'
                      ? view.state.own_character.portraitUrl
                      : null,
                  portraitAssetId:
                    typeof view.state.own_character?.portraitAssetId === 'string'
                      ? view.state.own_character.portraitAssetId
                      : null,
                  portraitMediaRef: view.state.own_character?.portraitMediaRef
                    ? parseMediaReference(view.state.own_character.portraitMediaRef)
                    : null,
                  mediaScope:
                    view.status === 'connected'
                      ? {
                          kind: 'room',
                          room_id: connection.room.room_id,
                          member_id: connection.member_id,
                          epoch: view.state.room.epoch ?? connection.room.epoch ?? 1,
                        }
                      : null,
                }}
                disabled={
                  view.status !== 'connected' ||
                  view.busy ||
                  view.pending !== null ||
                  view.state.agent_busy
                }
                onAction={async (encounter_id, expected_revision, action) => {
                  await controller.perform({
                    kind: 'combat',
                    request_id: crypto.randomUUID(),
                    encounter_id,
                    expected_revision,
                    action,
                  });
                  return (
                    controller.getSnapshot().errorCode === null &&
                    controller.getSnapshot().pending === null
                  );
                }}
              />
            )
          }
          onLeave={() => void leave()}
          onReconnect={() => void controller.reconnect()}
          onRetry={() => void controller.retryPending()}
          onCancel={() => controller.cancelDm()}
          onSend={(text) => controller.sendDm(text)}
          onAction={(action) => void controller.perform(action)}
          onHero={() => {
            useStore.getState().charCreation.resetDraft();
            setWizard(true);
          }}
        />
      </GameTableExperience>
      {wizard && (
        <CharacterWizard
          mode="initial"
          draftScope={{ kind: 'room', roomId: connection.room.room_id }}
          onClose={() => setWizard(false)}
          completeSoloOnboarding={false}
          allowCancel
          onCommitCharacter={submitCharacter}
        />
      )}
    </>
  );
}
export function RoomExperience({
  campaigns,
  onBack,
  campaignsLoading,
  campaignsError,
  onRefreshCampaigns,
}: Props) {
  const [saved, setSaved] = useState<GuestRecoveryChoice[]>([]);
  const [connection, setConnection] = useState<RoomConnection | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  useEffect(() => {
    let active = true;
    void savedGuestRooms()
      .then((choices) => {
        if (active) setSaved(choices);
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, []);
  const hostAttempt = useRef<{ signature: string; request: HostRoomRequest } | null>(null);
  const joinAttempt = useRef<{ signature: string; request: JoinRoomRequest } | null>(null);
  if (connection) return <ConnectedRoom connection={connection} onBack={onBack} />;
  return (
    <RoomHome
      campaignsLoading={campaignsLoading}
      campaignsError={campaignsError}
      onRefreshCampaigns={onRefreshCampaigns}
      campaigns={campaigns}
      busy={busy}
      error={error}
      onBack={onBack}
      saved={saved}
      onPostpone={() => {
        if (busy) return;
        setBusy(true);
        setError(false);
        void postponeGuestRoom()
          .then(() => savedGuestRooms())
          .then(setSaved)
          .catch(() => setError(true))
          .finally(() => setBusy(false));
      }}
      onSaved={(choice) => {
        if (busy) return;
        setBusy(true);
        setError(false);
        void resumeGuestRoom(choice)
          .then(setConnection)
          .catch(() => setError(true))
          .finally(() => setBusy(false));
      }}
      onRecoverHost={() => {
        if (busy) return;
        setBusy(true);
        setError(false);
        void recoverHostRoom()
          .then((recovered) => {
            if (recovered) setConnection(recovered);
            else setError(true);
          })
          .catch(() => setError(true))
          .finally(() => setBusy(false));
      }}
      onRecover={() => {
        if (busy) return;
        setBusy(true);
        setError(false);
        void recoverGuestRoom()
          .then((recovered) => {
            if (recovered) setConnection(recovered);
            else setError(true);
          })
          .catch(() => setError(true))
          .finally(() => setBusy(false));
      }}
      onHost={(campaign, bind) => {
        if (busy) return;
        setBusy(true);
        setError(false);
        void (async () => {
          const signature = JSON.stringify([campaign.id, bind]);
          if (hostAttempt.current?.signature !== signature) {
            const current = await fetchCampaignState(campaign.id);
            hostAttempt.current = {
              signature,
              request: {
                request_id: crypto.randomUUID(),
                campaign_id: campaign.id,
                session_id: current.session_id,
                expected_revision: current.campaign.revision,
                bind_addr: bind,
              },
            };
          }
          setConnection(await hostRoom(hostAttempt.current.request));
        })()
          .catch((cause) => {
            if (
              typeof cause === 'object' &&
              cause !== null &&
              'code' in cause &&
              [
                'native_room_bind_failed',
                'native_room_create_failed',
                'native_host_request_superseded',
              ].includes(String(cause.code))
            )
              hostAttempt.current = null;
            setError(true);
          })
          .finally(() => setBusy(false));
      }}
      onJoin={(address, room, name, invitation) => {
        if (busy) return;
        setBusy(true);
        setError(false);
        const signature = JSON.stringify([address, room, name, invitation]);
        if (joinAttempt.current?.signature !== signature)
          joinAttempt.current = {
            signature,
            request: {
              address,
              room_id: room,
              join_id: crypto.randomUUID(),
              display_name: name,
              invitation,
            },
          };
        void joinRoom(joinAttempt.current.request)
          .then(setConnection)
          .catch(() => setError(true))
          .finally(() => setBusy(false));
      }}
    />
  );
}
