/**
 * Valibot schema mirroring CharacterDraft + activeTab for persistence.
 * Used by persistStorage to validate the stored char_creation_draft JSON.
 */
import * as v from 'valibot';
import { DefenseChoiceSchema } from '../api/contracts/defense';
import { MediaReferenceSchema } from '../api/contracts/mediaAsset';
import { CURRENT_CHARACTER_RULESET_ID } from './charCreation';
import { AbilityScoresSchema, InventoryItemSchema } from './sharedSchemas';

const AbilityMethodSchema = v.nullable(
  v.picklist(['point_buy', 'standard_array', '4d6_drop_lowest']),
);

const EquipmentModeSchema = v.nullable(v.picklist(['package', 'gold']));

const EquipmentSlotSchema = v.object({
  slotId: v.string(),
  category: v.picklist(['weapon', 'armor', 'gear', 'pack', 'ammo', 'focus']),
  itemId: v.nullable(v.string()),
  customName: v.nullable(v.string()),
  fromBackground: v.boolean(),
  resolvedItemIds: v.array(v.string()),
});

const PersonalityFlagSchema = v.object({
  slotId: v.picklist([
    'bg-trait',
    'bg-bond',
    'align-trait',
    'align-quirk',
    'race-trait',
    'race-quirk',
  ]),
  source: v.picklist(['background', 'alignment', 'race']),
  flag: v.string(),
});

const WizardTabSchema = v.picklist([
  'class',
  'race',
  'background',
  'abilities',
  'skills',
  'spells',
  'equipment',
  'persona',
  'portrait',
  'review',
]);

// Strict schema (all fields required) - useful for full-draft validation.
export const StrictCharCreationDraftSchema = v.object({
  rulesetId: v.literal(CURRENT_CHARACTER_RULESET_ID),
  classId: v.nullable(v.string()),
  subclassId: v.nullable(v.string()),
  raceId: v.nullable(v.string()),
  subraceId: v.nullable(v.string()),
  backgroundId: v.nullable(v.string()),
  abilityMethod: AbilityMethodSchema,
  abilities: AbilityScoresSchema,
  abilityRollHistory: v.array(v.array(v.number())),
  pointBuyRemaining: v.number(),
  skillProfs: v.array(v.string()),
  spells: v.object({
    cantrips: v.array(v.string()),
    level1: v.array(v.string()),
  }),
  defenseChoice: v.nullable(DefenseChoiceSchema),
  equipmentMode: EquipmentModeSchema,
  equipmentSlots: v.array(EquipmentSlotSchema),
  equipmentInventory: v.array(InventoryItemSchema),
  goldRemaining: v.number(),
  personalityFlags: v.array(PersonalityFlagSchema),
  ideals: v.string(),
  bonds: v.string(),
  flaws: v.string(),
  backstory: v.string(),
  name: v.string(),
  alignment: v.nullable(v.string()),
  portraitUrl: v.nullable(v.string()),
  portraitAssetId: v.optional(v.nullable(v.pipe(v.string(), v.maxLength(160)))),
  portraitMediaRef: v.optional(v.nullable(MediaReferenceSchema)),
  mediaDraftId: v.optional(v.nullable(v.pipe(v.string(), v.uuid()))),
  draftScope: v.optional(
    v.nullable(
      v.variant('kind', [
        v.strictObject({
          kind: v.literal('campaign'),
          campaignId: v.pipe(v.string(), v.uuid()),
          characterId: v.nullable(v.pipe(v.string(), v.uuid())),
        }),
        v.strictObject({ kind: v.literal('room'), roomId: v.pipe(v.string(), v.uuid()) }),
      ]),
    ),
  ),
  portraitPrompt: v.nullable(v.string()),
  activeTab: WizardTabSchema,
});

// Ruleset identity is intentionally not forward-tolerant: development data from
// an older edition is discarded instead of being interpreted under SRD 5.2.1.
export const CharCreationDraftSchema = v.intersect([
  v.object({ rulesetId: v.literal(CURRENT_CHARACTER_RULESET_ID) }),
  v.partial(StrictCharCreationDraftSchema),
]);
