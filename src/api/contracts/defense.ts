import * as v from 'valibot';

export const DefenseChoiceSchema = v.strictObject({
  schema_version: v.literal(1),
  armor_id: v.nullable(v.string()),
  shield: v.boolean(),
});
export type DefenseChoice = v.InferOutput<typeof DefenseChoiceSchema>;
