import { useRef, useState } from 'react';
import { fetchCampaignState } from '../api/campaigns';
import type { CampaignSummary } from '../api/contracts/campaign';
import {
  type HostRoomRequest,
  hostRoom,
  type JoinRoomRequest,
  joinRoom,
  type RoomConnection,
  stopHostRoom,
} from '../api/rooms';
import { CharacterWizard } from '../components/CharacterWizard';
import { RoomCombatPanel } from '../features/combat/RoomCombatPanel';
import { RoomHome } from '../features/rooms/RoomHome';
import { RoomPlay } from '../features/rooms/RoomPlay';
import { useRoom } from '../features/rooms/useRoom';
import { useStore } from '../state/useStore';
import { roomPorts } from './roomPorts';

interface Props {
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
  const [wizard, setWizard] = useState(false);
  const [closing, setClosing] = useState(false);
  const [closingError, setClosingError] = useState(false);
  async function leave() {
    if (closing) return;
    setClosing(true);
    setClosingError(false);
    try {
      if (connection.kind === 'host') await stopHostRoom(connection.room.room_id);
      await onBack();
      controller.leave();
    } catch {
      setClosingError(true);
      setClosing(false);
    }
  }
  return (
    <>
      <RoomPlay
        view={{
          ...view,
          busy: view.busy || closing,
          errorCode: closingError ? 'native_host_stop_failed' : view.errorCode,
        }}
        battle={
          view.state && (
            <RoomCombatPanel
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
        onSend={(text) => void controller.sendDm(text)}
        onAction={(action) => void controller.perform(action)}
        onHero={() => {
          useStore.getState().charCreation.resetDraft();
          setWizard(true);
        }}
      />
      {wizard && (
        <CharacterWizard
          mode="initial"
          onClose={() => setWizard(false)}
          allowCancel
          onCommitCharacter={(character) => controller.submitCharacter(character)}
        />
      )}
    </>
  );
}
export function RoomExperience({ campaigns, onBack }: Props) {
  const [connection, setConnection] = useState<RoomConnection | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const hostAttempt = useRef<{ signature: string; request: HostRoomRequest } | null>(null);
  const joinAttempt = useRef<{ signature: string; request: JoinRoomRequest } | null>(null);
  if (connection) return <ConnectedRoom connection={connection} onBack={onBack} />;
  return (
    <RoomHome
      campaigns={campaigns}
      busy={busy}
      error={error}
      onBack={onBack}
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
