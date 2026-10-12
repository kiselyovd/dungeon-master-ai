import * as v from 'valibot';

export const MAX_MEDIA_BYTES = 8 * 1024 * 1024;
const uuid = v.pipe(v.string(), v.uuid());
const positive = v.pipe(v.number(), v.integer(), v.minValue(1));
export const MediaDescriptorSchema = v.pipe(
  v.strictObject({
    sha256: v.pipe(v.string(), v.regex(/^[a-f0-9]{64}$/)),
    mime_type: v.picklist(['image/png', 'image/jpeg', 'image/webp']),
    byte_len: v.pipe(positive, v.maxValue(MAX_MEDIA_BYTES)),
    width: v.pipe(positive, v.maxValue(8192)),
    height: v.pipe(positive, v.maxValue(8192)),
  }),
  v.check((value) => value.width * value.height <= 32 * 1024 * 1024),
);
export const MediaReferenceSchema = v.variant('source', [
  v.strictObject({
    source: v.literal('bundled'),
    asset_id: v.pipe(v.string(), v.minLength(1), v.maxLength(128), v.regex(/^[a-zA-Z0-9._-]+$/)),
    version: v.pipe(positive, v.maxValue(65535)),
  }),
  v.strictObject({
    source: v.literal('owned'),
    asset_id: uuid,
    category: v.picklist(['portrait', 'map', 'illustration']),
    origin: v.picklist(['generated', 'uploaded', 'migrated']),
    descriptor: MediaDescriptorSchema,
  }),
]);
export type MediaReference = v.InferOutput<typeof MediaReferenceSchema>;
export type OwnedMediaReference = Extract<MediaReference, { source: 'owned' }>;
export type LocalMediaScope =
  | { kind: 'draft'; draft_id: string }
  | { kind: 'campaign'; campaign_id: string; actor_id?: string | null };
export type MediaScope =
  | LocalMediaScope
  | { kind: 'room'; room_id: string; member_id?: string; epoch?: number };
export function parseMediaReference(value: unknown): MediaReference {
  const result = v.safeParse(MediaReferenceSchema, value);
  if (!result.success) throw new Error('media_asset_invalid');
  return result.output;
}
