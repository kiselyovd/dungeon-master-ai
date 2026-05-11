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
  srd_section: string;
}

export interface Class {
  id: string;
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
  level_1_features: unknown;
  spellcasting: unknown;
  subclass_at_level?: number;
  subclasses: unknown;
  source_url: string;
}

export interface BackgroundFeature {
  name_en: string;
  name_ru: string;
  description: string;
}

export interface Background {
  id: string;
  name_en: string;
  name_ru: string;
  skill_proficiencies: string[];
  tool_proficiencies: string[];
  language_proficiencies: unknown;
  starting_equipment: unknown;
  starting_gold?: number;
  feature: BackgroundFeature;
  suggested_characteristics: unknown;
  source_url?: string;
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
  name_en: string;
  name_ru: string;
  category: string;
  cost: Cost;
  damage: WeaponDamage;
  weight_lb: number;
  properties: string[];
  range_ft: WeaponRange;
  special_rules?: BilingualText;
  source_url: string;
  srd_section: string;
}

export interface Armor {
  id: string;
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
  srd_section: string;
}

export interface AdventuringGear {
  id: string;
  name_en: string;
  name_ru: string;
  cost: Cost;
  weight_lb: number;
  description_en?: string;
  description_ru?: string;
  source_url?: string;
  srd_section?: string;
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
  name_en: string;
  name_ru: string;
  prerequisites: FeatPrerequisite[];
  asi_grants: unknown;
  effects_en: string[];
  effects_ru: string[];
  mechanical_hooks: unknown;
  source_url: string;
  srd_section: string;
}

export interface WeaponProperty {
  id: string;
  name_en: string;
  name_ru: string;
  description_en: string;
  description_ru: string;
}

export interface Compendium {
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

export function fetchCompendium(): Promise<Compendium> {
  if (cached) return cached;
  cached = (async () => {
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
    return { races, classes, backgrounds, spells, equipment, feats, weapon_properties };
  })();
  return cached;
}

export function resetSrdCacheForTests(): void {
  cached = null;
}
