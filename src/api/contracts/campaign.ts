import * as v from 'valibot';

export const CAMPAIGN_SCHEMA_VERSION = 1 as const;
export const CAMPAIGN_RULESET_ID = 'srd-5.2.1' as const;

const UuidSchema = v.pipe(v.string(), v.uuid());
const TimestampSchema = v.pipe(v.string(), v.isoTimestamp());
const RevisionSchema = v.pipe(v.number(), v.integer(), v.minValue(1));

const CollaborativeSeedSchema = v.object({
  kind: v.literal('collaborative'),
  answers: v.record(v.string(), v.array(v.string())),
});
const DocumentSeedSchema = v.object({
  kind: v.literal('document'),
  document_id: UuidSchema,
});
const BundledSeedSchema = v.object({
  kind: v.literal('bundled'),
  adventure_id: v.pipe(v.string(), v.minLength(1)),
});

export const CampaignSeedSchema = v.variant('kind', [
  CollaborativeSeedSchema,
  DocumentSeedSchema,
  BundledSeedSchema,
]);

export const CampaignSummarySchema = v.object({
  id: UuidSchema,
  schema_version: v.literal(CAMPAIGN_SCHEMA_VERSION),
  ruleset_id: v.literal(CAMPAIGN_RULESET_ID),
  title: v.string(),
  premise: v.string(),
  last_session_id: UuidSchema,
  revision: RevisionSchema,
  updated_at: TimestampSchema,
});

export const CampaignSchema = v.object({
  ...CampaignSummarySchema.entries,
  seed: CampaignSeedSchema,
  created_at: TimestampSchema,
});

export const CampaignListSchema = v.array(CampaignSummarySchema);

export type CampaignSeed = v.InferOutput<typeof CampaignSeedSchema>;
export type CampaignSummary = v.InferOutput<typeof CampaignSummarySchema>;
export type Campaign = v.InferOutput<typeof CampaignSchema>;

export interface CreateCampaignInput {
  title: string;
  premise: string;
  seed: CampaignSeed;
}
