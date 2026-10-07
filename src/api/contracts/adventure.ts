import * as v from 'valibot';
import { ChatError } from '../errors';
import { BattleMapSchema } from './battleMap';

const LocationSchema = v.picklist(['town', 'quay', 'shrine', 'warehouse', 'archive', 'belfry']);
export const AdventureActionSchema = v.variant('kind', [
  v.object({ kind: v.literal('move'), location: LocationSchema }),
  v.object({
    kind: v.picklist([
      'inspect_quay',
      'start_defense',
      'talk_agata',
      'consult_leon',
      'negotiate_clapper',
      'retrieve_clapper_publicly',
      'repair_clapper',
      'locate_tikhon',
      'rescue_tikhon',
      'recover_roster',
      'recover_ledger',
      'acknowledge_echo',
      'restore_ward',
      'seal_sluice',
      'publish_ledger',
      'finish',
    ]),
  }),
]);
export const AdventureProjectionSchema = v.object({
  map: v.optional(v.nullable(BattleMapSchema)),
  state: v.object({
    package_id: v.literal('bell-beneath-water'),
    defense: v.optional(v.nullable(v.picklist(['active', 'held', 'retreated', 'defeated']))),
    defense_encounter_id: v.optional(v.nullable(v.pipe(v.string(), v.uuid()))),
    location: LocationSchema,
    facts: v.array(
      v.picklist([
        'ferry_route',
        'porter_observation',
        'ward_damaged',
        'ward_understood',
        'roster_available',
        'clapper_recovered',
        'clapper_repaired',
        'tikhon_located',
        'tikhon_rescued',
        'ledger_recovered',
        'ledger_published',
        'echo_acknowledged',
        'ward_restored',
        'lower_sluice_sealed',
        'reward_granted',
        'defense_held',
      ]),
    ),
    ending: v.nullable(v.picklist(['remembered_river', 'closed_crossing', 'unfinished_debt'])),
    closed: v.boolean(),
    history: v.array(AdventureActionSchema),
  }),
  available_actions: v.array(AdventureActionSchema),
});
export type AdventureAction = v.InferOutput<typeof AdventureActionSchema>;
export type AdventureProjection = v.InferOutput<typeof AdventureProjectionSchema>;

export const AdventureToolOutcomeSchema = v.object({
  projection: v.optional(v.nullable(v.unknown())),
  schema_version: v.literal(1),
  ruleset_id: v.literal('srd-5.2.1'),
  campaign_id: v.pipe(v.string(), v.uuid()),
  session_id: v.pipe(v.string(), v.uuid()),
  character_id: v.pipe(v.string(), v.uuid()),
  revision: v.pipe(v.number(), v.integer(), v.minValue(1), v.maxValue(Number.MAX_SAFE_INTEGER)),
  adventure: AdventureProjectionSchema,
  inventory: v.array(
    v.object({
      id: v.string(),
      name: v.string(),
      count: v.pipe(v.number(), v.integer(), v.minValue(0)),
      icon: v.optional(v.string()),
    }),
  ),
});

export function parseAdventureProjection(value: unknown): AdventureProjection | null {
  if (value == null) return null;
  const parsed = v.safeParse(AdventureProjectionSchema, value);
  if (!parsed.success) throw new ChatError('invalid_response', 'adventure projection is invalid');
  return parsed.output;
}
