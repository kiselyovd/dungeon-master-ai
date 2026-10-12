import * as v from 'valibot';
import { AdventureProjectionSchema } from './adventure';
import { CampaignSeedSchema } from './campaign';
import { MediaReferenceSchema } from './mediaAsset';
import { PublicHeroRecoverySchema } from './recovery';

const uuid = v.pipe(v.string(), v.uuid());
const integer = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(Number.MAX_SAFE_INTEGER));
const document = v.record(v.string(), v.unknown());
const role = v.picklist(['host', 'player']);
const RoomCampaignSchema = v.strictObject({
  id: uuid,
  schema_version: v.literal(1),
  ruleset_id: v.literal('srd-5.2.1'),
  title: v.string(),
  premise: v.string(),
  seed: CampaignSeedSchema,
  last_session_id: uuid,
  revision: integer,
  created_at_epoch_ms: integer,
  updated_at_epoch_ms: integer,
});
export const PublicRoomSchema = v.strictObject({
  epoch: v.optional(v.pipe(integer, v.minValue(1))),
  room_id: uuid,
  campaign_id: uuid,
  session_id: uuid,
  active: v.boolean(),
  members: v.array(
    v.strictObject({ member_id: uuid, display_name: v.string(), role, active: v.boolean() }),
  ),
});
export const PublicActorSchema = v.strictObject({
  member_id: uuid,
  character_id: uuid,
  role,
  name: v.string(),
  class_id: v.string(),
  level: integer,
  hp: integer,
  max_hp: integer,
  ac: integer,
  portrait_asset_id: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(128)))),
  portrait_media_ref: v.optional(v.nullable(MediaReferenceSchema)),
});
export const ProposalSummarySchema = v.strictObject({
  proposal_id: uuid,
  status: v.picklist(['pending', 'accepted', 'superseded']),
  actor: PublicActorSchema,
});
export const RoomStateSchema = v.pipe(
  v.strictObject({
    campaign: RoomCampaignSchema,
    room: PublicRoomSchema,
    title: v.string(),
    revision: integer,
    actors: v.array(PublicActorSchema),
    recovery: v.array(PublicHeroRecoverySchema),
    own_character: v.nullable(document),
    own_proposal: v.nullable(
      v.strictObject({ summary: ProposalSummarySchema, character: document }),
    ),
    pending_proposals: v.nullable(v.array(ProposalSummarySchema)),
    messages: v.array(
      v.strictObject({
        id: uuid,
        role: v.picklist(['user', 'assistant']),
        text: v.string(),
        truncated: v.boolean(),
      }),
    ),
    combat: v.nullable(v.unknown()),
    scene: v.nullable(v.unknown()),
    map_media_ref: v.optional(v.nullable(MediaReferenceSchema)),
    adventure: v.nullable(AdventureProjectionSchema),
    clock: v.strictObject({ elapsed_seconds: integer }),
    journal: v.array(v.unknown()),
    npcs: v.array(v.unknown()),
    agent_busy: v.boolean(),
  }),
  v.check(
    (state) =>
      state.room.campaign_id === state.campaign.id &&
      state.room.session_id === state.campaign.last_session_id &&
      state.recovery.length ===
        state.actors.filter((actor) =>
          state.room.members.some(
            (member) => member.member_id === actor.member_id && member.active,
          ),
        ).length &&
      new Set(state.recovery.map((hero) => hero.character_id)).size === state.recovery.length &&
      state.recovery.every((hero) => {
        const actor = state.actors.find((actor) => actor.character_id === hero.character_id);
        if (!actor || (hero.status === 'healthy' ? actor.hp <= 0 : actor.hp !== 0)) return false;
        return hero.actions.every((intent) => {
          if (state.combat !== null) return false;
          if (intent.kind === 'death_save')
            return (
              hero.status === 'dying' && hero.character_id === state.own_character?.characterId
            );
          return (
            intent.target_character_id === hero.character_id &&
            (intent.kind === 'aid' ? hero.status === 'dying' : hero.status === 'stable')
          );
        });
      }),
  ),
);
export const RoomUpdateSchema = v.pipe(
  v.strictObject({
    cursor: integer,
    head: integer,
    timeline_reset_cursor: integer,
    reset: v.boolean(),
    events: v.array(v.strictObject({ sequence: integer, kind: v.string(), payload: v.unknown() })),
    state: RoomStateSchema,
  }),
  v.check(
    (update) =>
      update.cursor <= update.head &&
      update.timeline_reset_cursor <= update.head &&
      update.events.every(
        (event, index) =>
          event.sequence <= update.cursor &&
          (index === 0 ||
            event.sequence > (update.events[index - 1]?.sequence ?? Number.MAX_SAFE_INTEGER)),
      ),
  ),
);
export type RoomUpdate = v.InferOutput<typeof RoomUpdateSchema>;
export type RoomState = v.InferOutput<typeof RoomStateSchema>;
export type PublicRoom = v.InferOutput<typeof PublicRoomSchema>;
export type RoomStreamOperation =
  | { kind: 'subscribe'; after: number | null }
  | {
      kind: 'dm_turn';
      request_id: string;
      player_message: string;
      resume: boolean;
      awaiting_clarification?: string | undefined;
      clarification?: { id: string; reply: string } | undefined;
    };
