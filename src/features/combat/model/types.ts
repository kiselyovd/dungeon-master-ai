export type AoeShape = 'cone' | 'sphere' | 'line' | 'cube';

import type { AdventureProjection } from '../../../api/contracts/adventure';
import type { BattleMap } from '../../../api/contracts/battleMap';

export interface SpellChoiceDto {
  id: string;
  level: number;
  range_ft: number;
  bonus_action: boolean;
  healing: boolean;
  supported: boolean;
}
export interface CastingProfileDto {
  level: number;
  ability_modifier: number;
  attack_bonus: number;
  save_dc: number;
  prepared: string[];
  choices: SpellChoiceDto[];
  slots: Record<string, { maximum: number; remaining: number }>;
}

export interface WeaponAttackProfileDto {
  weapon_id: string;
  attack_bonus: number;
  damage: { count: number; die: string; modifier: number };
  damage_type: string;
  reach_ft: number;
  normal_range_ft: number;
  long_range_ft: number;
  ranged: boolean;
  thrown: boolean;
  ammunition: boolean;
  two_handed: boolean;
  versatile_damage: { count: number; die: string; modifier: number } | null;
  heavy_disadvantage: boolean;
}
export interface CombatLoadoutDto {
  default_weapon: string;
  attacks: WeaponAttackProfileDto[];
  quantities: Record<string, number>;
  arrows: number;
  shield: boolean;
}

export interface AoeTemplateEntry {
  id: string;
  shape: AoeShape;
  originX: number;
  originY: number;
  sizeInFt: number;
  school: string;
  rotateDeg: number;
  expiresAt: number;
}

export interface CombatToken {
  opponentAppearance?: string;
  isDead?: boolean;
  displayNameKey?: string;
  id: string;
  characterId?: string;
  loadout?: CombatLoadoutDto;
  casting?: CastingProfileDto;
  spellSlotUsed?: boolean;
  name: string;
  hp: number;
  maxHp: number;
  ac: number;
  x: number;
  y: number;
  conditions: string[];
  isActive?: boolean;
  speed?: number;
  actionAvailable?: boolean;
  bonusAvailable?: boolean;
  reactionAvailable?: boolean;
  movementRemaining?: number;
}

export interface CombatProjectionDto {
  appearances?: Record<string, string>;
  authored?: {
    map_skin_asset_id?: string | null;
    kind: 'bell_archive_defense';
    map: BattleMap;
    outcome: 'active' | 'held' | 'retreated' | 'defeated' | null;
  } | null;
  adventure?: AdventureProjection | null;
  campaign_scope?: {
    campaign_id: string;
    session_id: string;
    character_id: string;
    revision: number;
  } | null;
  schema_version: number;
  encounter_id: string;
  revision: number;
  snapshot: {
    active: boolean;
    round: number;
    current_combatant: string | null;
    initiative: Array<{ id: string; roll: number; dex_tiebreak: number }>;
    combatants: Array<{
      id: string;
      display_name_key?: string | null;
      character_id?: string | null;
      loadout?: CombatLoadoutDto | null;
      casting?: CastingProfileDto | null;
      name: string;
      max_hp: number;
      current_hp: number;
      temp_hp: number;
      ac: number;
      speed_ft: number;
      initiative_roll: number;
      dex_mod: number;
      conditions: string[];
      budget: {
        spell_slot_used?: boolean;
        action: boolean;
        bonus_action: boolean;
        reaction: boolean;
        movement_ft: number;
      };
      is_dead: boolean;
      withdrawn?: boolean;
      position: { x: number; y: number };
    }>;
  };
  events: unknown[];
}

export interface SnapshotToken {
  id: string;
  name: string;
  hp: number;
  max_hp: number;
  ac: number;
  x: number;
  y: number;
  conditions: string[];
  resistances: string[];
  immunities: string[];
  vulnerabilities: string[];
}

export interface SnapshotCombat {
  active: boolean;
  encounter_id: string;
  round: number;
  current_turn_id: string | null;
  initiative: string[];
  tokens: SnapshotToken[];
}
