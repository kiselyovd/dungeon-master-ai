/**
 * Shared valibot schemas used by both charCreationSchema.ts and persistStorage.ts.
 * Centralised here to avoid duplication and keep the two consumers in sync.
 */
import * as v from 'valibot';

export const AbilityScoresSchema = v.object({
  str: v.number(),
  dex: v.number(),
  con: v.number(),
  int: v.number(),
  wis: v.number(),
  cha: v.number(),
});

export const InventoryItemSchema = v.object({
  id: v.string(),
  name: v.string(),
  count: v.number(),
  icon: v.optional(v.string()),
});
