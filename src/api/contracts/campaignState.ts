import * as v from 'valibot';
import { CampaignSchema } from './campaign';

const MessageSchema = v.object({
  role: v.string(),
  content: v.optional(v.string()),
  parts: v.optional(
    v.array(
      v.object({
        type: v.string(),
        text: v.optional(v.string()),
        mime: v.optional(v.string()),
        data_b64: v.optional(v.string()),
        name: v.optional(v.nullable(v.string())),
      }),
    ),
  ),
});

export const CampaignStateSchema = v.object({
  schema_version: v.literal(1),
  ruleset_id: v.literal('srd-5.2.1'),
  campaign: CampaignSchema,
  session_id: v.pipe(v.string(), v.uuid()),
  character_id: v.nullable(v.pipe(v.string(), v.uuid())),
  projection: v.object({
    messages: v.array(MessageSchema),
    character: v.record(v.string(), v.unknown()),
    combat: v.nullable(v.unknown()),
    scene: v.nullable(v.unknown()),
    journal: v.array(v.unknown()),
    npcs: v.array(v.unknown()),
  }),
});

export type CampaignStateResponse = v.InferOutput<typeof CampaignStateSchema>;

export interface CommitCharacterRequest {
  sessionId: string;
  characterId: string;
  expectedRevision: number;
  causationId: string;
  character: Record<string, unknown>;
}
