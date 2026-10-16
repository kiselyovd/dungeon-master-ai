import { LazyStore } from '@tauri-apps/plugin-store';
import * as v from 'valibot';
import { CampaignSeedSchema, type CreateCampaignInput } from '../api/contracts/campaign';
import type { CommitCharacterRequest } from '../api/contracts/campaignState';

// Non-secret drafts live separately from settings. A successful disk ACK is
// mandatory before a mutation is sent. Never store portrait bytes here.
const store = new LazyStore('campaign-preparation.json');
const uuid = v.pipe(v.string(), v.uuid());
const text = v.pipe(v.string(), v.maxLength(16_000));
const DraftSchema = v.object({
  source: v.picklist(['bundled', 'collaborative']),
  title: text,
  premise: text,
});
export type CampaignCreationDraft = v.InferOutput<typeof DraftSchema>;
const CreationSchema = v.object({
  requestId: uuid,
  title: text,
  premise: text,
  seed: CampaignSeedSchema,
});
const CharacterSchema = v.object({
  campaignId: uuid,
  sessionId: uuid,
  characterId: uuid,
  expectedRevision: v.pipe(v.number(), v.integer(), v.minValue(1)),
  causationId: v.pipe(v.string(), v.minLength(1), v.maxLength(256)),
  documentHash: v.pipe(v.string(), v.regex(/^[a-f0-9]{64}$/)),
});
let queue: Promise<unknown> = Promise.resolve();
function serial<T>(work: () => Promise<T>): Promise<T> {
  const next = queue.then(work, work);
  queue = next.catch(() => undefined);
  return next;
}
async function read<T>(key: string, schema: v.GenericSchema<unknown, T>): Promise<T | null> {
  const raw = await store.get<unknown>(key);
  if (raw == null) return null;
  const result = v.safeParse(schema, raw);
  // Retain corrupt data rather than overwriting an unknown mutation identity.
  if (!result.success) throw new Error('campaign_preparation_invalid');
  return result.output;
}
async function save(key: string, value: unknown) {
  await store.set(key, value);
  await store.save();
}
async function removeAcknowledged(keys: string[]) {
  const previous = await Promise.all(keys.map((key) => store.get<unknown>(key)));
  try {
    for (const key of keys) await store.delete(key);
    await store.save();
  } catch (error) {
    // The plugin has a write-back cache. Keep the original identity in that
    // cache when its disk flush fails, so the next retry cannot invent one.
    for (const [index, key] of keys.entries()) {
      if (previous[index] !== undefined) await store.set(key, previous[index]);
    }
    throw error;
  }
}
export function readCampaignCreationDraft() {
  return serial(() => read('draft', DraftSchema));
}
export function saveCampaignCreationDraft(draft: CampaignCreationDraft) {
  return serial(() => save('draft', v.parse(DraftSchema, draft)));
}
export function prepareCampaignCreation(input: CreateCampaignInput) {
  return serial(async () => {
    const normalized = {
      title: input.title.trim(),
      premise: input.premise.trim(),
      seed: input.seed,
    };
    const existing = await read('creation', CreationSchema);
    if (
      existing &&
      ((existing.seed.kind === 'bundled' &&
        input.seed.kind === 'bundled' &&
        existing.seed.adventure_id === input.seed.adventure_id) ||
        JSON.stringify({
          title: existing.title,
          premise: existing.premise,
          seed: existing.seed,
        }) === JSON.stringify(normalized))
    ) {
      await save('creation', existing);
      return existing;
    }
    const attempt = v.parse(CreationSchema, { ...normalized, requestId: crypto.randomUUID() });
    await save('creation', attempt);
    return attempt;
  });
}
export function acknowledgeCampaignCreation(requestId: string) {
  return serial(async () => {
    if ((await read('creation', CreationSchema))?.requestId !== requestId) return;
    await removeAcknowledged(['creation', 'draft']);
  });
}
export async function prepareCampaignCharacter(
  campaignId: string,
  input: Omit<CommitCharacterRequest, 'causationId' | 'characterId'> & {
    characterId: string | null;
  },
): Promise<CommitCharacterRequest> {
  const bytes = new TextEncoder().encode(JSON.stringify(input.character));
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  const documentHash = Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, '0'),
  ).join('');
  return serial(async () => {
    const key = `character:${campaignId}`;
    const existing = await read(key, CharacterSchema);
    const attempt =
      existing?.sessionId === input.sessionId && existing.documentHash === documentHash
        ? existing
        : v.parse(CharacterSchema, {
            campaignId,
            sessionId: input.sessionId,
            documentHash,
            characterId: input.characterId ?? crypto.randomUUID(),
            expectedRevision: input.expectedRevision,
            causationId: `character-${crypto.randomUUID()}`,
          });
    await save(key, attempt);
    return {
      sessionId: attempt.sessionId,
      characterId: attempt.characterId,
      expectedRevision: attempt.expectedRevision,
      causationId: attempt.causationId,
      character: input.character,
    };
  });
}
export function acknowledgeCampaignCharacter(campaignId: string, causationId: string) {
  return serial(async () => {
    const key = `character:${campaignId}`;
    if ((await read(key, CharacterSchema))?.causationId !== causationId) return;
    await removeAcknowledged([key]);
  });
}
