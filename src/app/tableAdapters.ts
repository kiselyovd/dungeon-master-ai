import type { BattleMap } from '../api/contracts/battleMap';
import type { Campaign } from '../api/contracts/campaign';
import type { MediaReference } from '../api/contracts/mediaAsset';
import type { RoomConnection } from '../api/rooms';
import { publicToolOutcome } from '../features/agent-turn/model/publicToolOutcome';
import { parseCombatProjection } from '../features/combat/model/combatProjection';
import type { CombatToken } from '../features/combat/model/types';
import type {
  TableActor,
  TableAvailability,
  TablePresentation,
  TableScene,
} from '../features/game-table';
import type { RoomView } from '../features/rooms/model/RoomController';
import type { ChatMessage } from '../state/chat';
import type { PcData } from '../state/pc';
import type { ToolLogEntry } from '../state/toolLog';

function scene(
  title: string | null,
  map: BattleMap | null,
  art: MediaReference | null,
): TableScene {
  return map ? { kind: 'tactical', title, map, art } : { kind: 'exploration', title, art };
}
function blocked(
  reason: Exclude<TableAvailability, { enabled: true }>['reason'],
): TableAvailability {
  return { enabled: false, reason };
}
export interface SoloTableInput {
  campaign: Campaign;
  timeline: number;
  character: PcData;
  title: string | null;
  map: BattleMap | null;
  art: MediaReference | null;
  tokens: readonly CombatToken[];
  activeFight: boolean;
  encounterId: string | null;
  messages: readonly ChatMessage[];
  outcomes: readonly ToolLogEntry[];
  configured: boolean;
  runtimeReady: boolean | null;
  busy: boolean;
  recovery: TablePresentation['recovery'];
}
/** Composition adapters project allowlisted public fields. No commands, store
 * writes, provider selection or domain calculations occur here. */
export function soloTable(input: SoloTableInput): TablePresentation {
  const pc = input.character;
  const ownId = pc.characterId ?? null;
  const actors: TableActor[] =
    ownId && pc.heroClass
      ? [
          {
            id: ownId,
            characterId: ownId,
            name: pc.name ?? '',
            classId: pc.heroClass,
            allegiance: 'self',
            hp: pc.hp,
            maxHp: pc.hpMax,
            ac: pc.ac,
            life: 'unknown',
            portraitAssetId: pc.portraitAssetId ?? null,
            portraitMediaRef: pc.portraitMediaRef ?? null,
          },
        ]
      : [];
  for (const token of input.tokens) {
    if (token.characterId === ownId || token.id === ownId) continue;
    actors.push({
      id: token.characterId ?? token.id,
      characterId: token.characterId ?? null,
      name: token.name,
      classId: null,
      allegiance: token.characterId ? 'party' : 'enemy',
      hp: token.hp,
      maxHp: token.maxHp,
      ac: token.ac,
      life: token.isDead ? 'dead' : 'unknown',
      portraitAssetId: null,
      portraitMediaRef: null,
    });
  }
  const readiness = !input.configured
    ? 'needs_model'
    : input.runtimeReady === null
      ? 'unknown'
      : input.runtimeReady
        ? 'ready'
        : 'needs_model';
  const availability = !actors.some((actor) => actor.id === ownId)
    ? blocked('hero_required')
    : input.busy
      ? blocked('busy')
      : input.recovery !== 'clear'
        ? blocked('recovery')
        : readiness !== 'ready'
          ? blocked('model_required')
          : { enabled: true as const };
  return {
    scope: {
      mode: 'solo',
      campaignId: input.campaign.id,
      sessionId: input.campaign.last_session_id,
      revision: input.campaign.revision,
      timeline: input.timeline,
      roomId: null,
      roomEpoch: null,
      encounterId: input.encounterId,
    },
    title: input.campaign.title,
    scene: scene(input.title, input.map, input.art),
    actors,
    ownActorId: actors.some((a) => a.id === ownId) ? ownId : null,
    messages: input.messages
      .filter((message) => message.role !== 'system')
      .map((message) => ({
        id: message.id,
        role: message.role as 'user' | 'assistant',
        text: message.content,
        delivery: 'confirmed',
        truncated: false,
      })),
    outcomes: input.outcomes.map((entry) => ({
      id: entry.id,
      ...publicToolOutcome(entry.toolName, entry.result),
      failed: entry.isError,
    })),
    connection: 'local',
    readiness,
    recovery: input.recovery,
    combat: input.activeFight ? 'active' : input.encounterId ? 'ended' : 'inactive',
    availability: {
      send: availability,
      createHero: input.busy ? blocked('busy') : { enabled: true },
      configure: input.busy ? blocked('busy') : { enabled: true },
    },
  };
}
export function roomTable(view: RoomView, connection: RoomConnection): TablePresentation {
  const state = view.state;
  const own =
    state?.actors.find((actor) => actor.member_id === connection.member_id)?.character_id ?? null;
  const combat = parseCombatProjection(state?.combat);
  const map = combat?.authored?.map ?? state?.adventure?.map ?? null;
  const rawScene = state?.scene;
  const title =
    rawScene &&
    typeof rawScene === 'object' &&
    'title' in rawScene &&
    typeof rawScene.title === 'string'
      ? rawScene.title
      : null;
  const recovery: TablePresentation['recovery'] =
    view.recoveryStatus === 'loading'
      ? 'checking'
      : view.recoveryStatus === 'error'
        ? 'unavailable'
        : view.recovery || view.pending
          ? 'pending'
          : 'clear';
  const availability =
    view.status === 'disconnected'
      ? blocked('offline')
      : view.status !== 'connected' || !state
        ? blocked('syncing')
        : !own || !state.own_character
          ? blocked('hero_required')
          : view.busy || state.agent_busy
            ? blocked('busy')
            : recovery !== 'clear'
              ? blocked('recovery')
              : { enabled: true as const };
  return {
    scope: {
      mode: connection.kind,
      campaignId: state?.campaign.id ?? connection.room.campaign_id,
      sessionId: state?.campaign.last_session_id ?? connection.room.session_id,
      revision: state?.revision ?? 0,
      timeline: view.timelineReset,
      roomId: connection.room.room_id,
      roomEpoch: state?.room.epoch ?? connection.room.epoch ?? 1,
      encounterId: combat?.encounter_id ?? null,
    },
    title: state?.title ?? '',
    scene: scene(title, map, state?.map_media_ref ?? null),
    actors:
      state?.actors.map((actor) => ({
        id: actor.character_id,
        characterId: actor.character_id,
        name: actor.name,
        classId: actor.class_id,
        allegiance: actor.character_id === own ? 'self' : 'party',
        hp: actor.hp,
        maxHp: actor.max_hp,
        ac: actor.ac,
        life:
          state.recovery.find((item) => item.character_id === actor.character_id)?.status ??
          'unknown',
        portraitAssetId: actor.portrait_asset_id ?? null,
        portraitMediaRef: actor.portrait_media_ref ?? null,
      })) ?? [],
    ownActorId: own,
    messages: [
      ...(state?.messages.map((message) => ({
        id: message.id,
        role: message.role,
        text: message.text,
        delivery: 'confirmed' as const,
        truncated: message.truncated,
      })) ?? []),
      ...(view.dmText
        ? [
            {
              id: 'streaming-dm',
              role: 'assistant' as const,
              text: view.dmText,
              delivery: 'streaming' as const,
              truncated: false,
            },
          ]
        : []),
    ],
    outcomes: [],
    connection:
      view.status === 'connected'
        ? 'connected'
        : view.status === 'disconnected'
          ? 'disconnected'
          : 'connecting',
    readiness: 'unknown',
    recovery,
    combat: combat?.snapshot.active ? 'active' : combat ? 'ended' : 'inactive',
    availability: {
      send: availability,
      createHero: view.busy ? blocked('busy') : { enabled: true },
      configure:
        connection.kind !== 'host'
          ? blocked('host_only')
          : view.busy || state?.agent_busy
            ? blocked('busy')
            : { enabled: true },
    },
  };
}
