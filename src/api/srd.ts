/**
 * SRD compendium API client.
 *
 * Mirrors the seven /srd/* read-only endpoints (`crates/app-server/src/routes/srd.rs`).
 * The whole compendium is small enough to fetch once at app boot and cache for
 * the lifetime of the page. Repeat callers share one batch via a module-level
 * promise.
 */

import { backendUrl } from './client';
import { ChatError } from './errors';

export const CURRENT_RULESET_ID = 'srd-5.2.1' as const;

export interface CatalogEntryIdentity {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  source_page: number;
  srd_section: string;
}

// --- Mirror types ---------------------------------------------------------

export interface AbilityScores {
  str?: number;
  dex?: number;
  con?: number;
  int?: number;
  wis?: number;
  cha?: number;
}

export interface Senses {
  darkvision_ft?: number;
  truesight_ft?: number;
}

export interface AgeRange {
  mature_at: number;
  max_lifespan: number;
}

export interface RaceProficiencies {
  skills: string[];
  weapons: string[];
  tools: string[];
  saves: string[];
}

export interface RaceTrait {
  id: string;
  name_en: string;
  name_ru: string;
  mechanical_description: string;
  flavor_description?: string;
}

export interface Subrace {
  id: string;
  name_en: string;
  name_ru: string;
  additional_asi: AbilityScores;
  additional_traits: RaceTrait[];
}

export interface Race {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  ability_score_increases: AbilityScores;
  age: AgeRange;
  alignment_tendency?: string;
  size: string;
  speed: number;
  languages: string[];
  proficiencies: RaceProficiencies;
  senses: Senses;
  traits: RaceTrait[];
  subraces: Subrace[];
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface Class {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  hit_die: number;
  primary_ability: string[];
  saving_throw_proficiencies: string[];
  armor_proficiencies: string[];
  weapon_proficiencies: string[];
  tool_proficiencies: string[];
  skill_proficiencies: unknown;
  starting_equipment: unknown;
  starting_gold?: number;
  level_1_features: unknown;
  spellcasting: unknown;
  subclass_at_level?: number;
  subclasses: unknown;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface BackgroundFeature {
  name_en: string;
  name_ru: string;
  description: string;
}

export interface Background {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  skill_proficiencies: string[];
  tool_proficiencies: string[];
  language_proficiencies: unknown;
  starting_equipment: unknown;
  starting_gold?: number;
  ability_score_options: string[];
  feat_id: string;
  feature: BackgroundFeature;
  suggested_characteristics: unknown;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface SpellComponents {
  v: boolean;
  s: boolean;
  m?: string;
}

export interface SpellDamage {
  dice: string;
  type: string;
}

export interface SpellAttack {
  kind: string;
}

export interface SpellSave {
  ability: string;
  half_on_success: boolean;
}

export interface BilingualText {
  en?: string;
  ru?: string;
}

export interface Spell {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  level: number;
  school: string;
  casting_time: string;
  range_ft: unknown;
  components: SpellComponents;
  duration: string;
  ritual: boolean;
  concentration: boolean;
  classes: string[];
  description_en: string;
  description_ru: string;
  at_higher_levels?: BilingualText;
  damage?: SpellDamage;
  attack?: SpellAttack;
  save?: SpellSave;
  scales_with_level?: string;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface Cost {
  gp?: number;
  sp?: number;
  cp?: number;
}

export interface WeaponRange {
  normal?: number;
  long?: number;
}

export interface WeaponDamage {
  dice: string;
  type: string;
  versatile_dice?: string;
}

export interface Weapon {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  category: string;
  cost: Cost;
  damage: WeaponDamage;
  weight_lb: number;
  properties: string[];
  mastery?: string;
  range_ft: WeaponRange;
  special_rules?: BilingualText;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface Armor {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  category: string;
  cost: Cost;
  ac_base: number;
  ac_dex_bonus_cap?: number;
  stealth_disadvantage: boolean;
  str_req?: number;
  weight_lb: number;
  don_time?: string;
  doff_time?: string;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface AdventuringGear {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  cost: Cost;
  weight_lb: number;
  description_en?: string;
  description_ru?: string;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface EquipmentResponse {
  weapons: Weapon[];
  armor: Armor[];
  adventuring_gear: AdventuringGear[];
}

export interface FeatPrerequisite {
  type: string;
  ability?: string;
  minimum?: number;
}

export interface Feat {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  prerequisites: FeatPrerequisite[];
  asi_grants: unknown;
  effects_en: string[];
  effects_ru: string[];
  mechanical_hooks: unknown;
  source_url: string;
  source_page: number;
  srd_section: string;
}

export interface WeaponProperty {
  id: string;
  ruleset_id: typeof CURRENT_RULESET_ID;
  name_key: string;
  name_en: string;
  name_ru: string;
  description_en: string;
  description_ru: string;
  source_page: number;
  srd_section: string;
}

export interface Compendium {
  rulesetId: typeof CURRENT_RULESET_ID;
  races: Race[];
  classes: Class[];
  backgrounds: Background[];
  spells: Spell[];
  equipment: EquipmentResponse;
  feats: Feat[];
  weapon_properties: WeaponProperty[];
}

// --- Client (in-memory cache; single flight) -----------------------------

let cached: Promise<Compendium> | null = null;

async function fetchJson<T>(path: string): Promise<T> {
  const url = await backendUrl(path);
  const resp = await fetch(url);
  if (!resp.ok) throw new ChatError('http_error', `SRD fetch ${path} failed: ${resp.status}`);
  return (await resp.json()) as T;
}

function assertCurrentRuleset(entries: CatalogEntryIdentity[], kind: string): void {
  const seen = new Set<string>();
  for (const entry of entries) {
    if (entry.ruleset_id !== CURRENT_RULESET_ID) {
      throw new ChatError('invalid_response', `${kind} ruleset mismatch`);
    }
    if (seen.has(entry.id)) {
      throw new ChatError('invalid_response', `${kind} duplicate identifier`);
    }
    if (!entry.name_key.startsWith('catalog.')) {
      throw new ChatError('invalid_response', `${kind} locale key missing`);
    }
    seen.add(entry.id);
  }
}

export function fetchCompendium(): Promise<Compendium> {
  if (cached) return cached;
  const pending = (async () => {
    const [races, classes, backgrounds, spells, equipment, feats, weapon_properties] =
      await Promise.all([
        fetchJson<Race[]>('/srd/races'),
        fetchJson<Class[]>('/srd/classes'),
        fetchJson<Background[]>('/srd/backgrounds'),
        fetchJson<Spell[]>('/srd/spells'),
        fetchJson<EquipmentResponse>('/srd/equipment'),
        fetchJson<Feat[]>('/srd/feats'),
        fetchJson<WeaponProperty[]>('/srd/weapon-properties'),
      ]);
    assertCurrentRuleset(races, 'species');
    assertCurrentRuleset(classes, 'classes');
    assertCurrentRuleset(backgrounds, 'backgrounds');
    assertCurrentRuleset(spells, 'spells');
    assertCurrentRuleset(equipment.weapons, 'weapons');
    assertCurrentRuleset(equipment.armor, 'armor');
    assertCurrentRuleset(equipment.adventuring_gear, 'adventuring gear');
    assertCurrentRuleset(feats, 'feats');
    assertCurrentRuleset(weapon_properties, 'weapon properties');
    return {
      rulesetId: CURRENT_RULESET_ID,
      races,
      classes,
      backgrounds,
      spells,
      equipment,
      feats,
      weapon_properties,
    };
  })();
  cached = pending;
  // Clear the cache on failure so a retry re-attempts instead of replaying the
  // rejected promise forever (the wizard's load-error + retry path depends on
  // this). Callers still receive the original rejection from `pending`.
  pending.catch(() => {
    if (cached === pending) cached = null;
  });
  return pending;
}

export function resetSrdCacheForTests(): void {
  cached = null;
}
