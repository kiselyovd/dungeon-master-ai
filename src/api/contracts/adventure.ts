import * as v from 'valibot';
import { ChatError } from '../errors';

const LocationSchema = v.picklist(['town', 'quay', 'shrine', 'warehouse', 'archive', 'belfry']);
export const AdventureActionSchema = v.variant('kind', [
  v.object({ kind: v.literal('move'), location: LocationSchema }),
  v.object({
    kind: v.picklist([
      'inspect_quay',
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
  state: v.object({
    package_id: v.literal('bell-beneath-water'),
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

export function parseAdventureProjection(value: unknown): AdventureProjection | null {
  if (value == null) return null;
  const parsed = v.safeParse(AdventureProjectionSchema, value);
  if (!parsed.success) throw new ChatError('invalid_response', 'adventure projection is invalid');
  return parsed.output;
}
