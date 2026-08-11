export type AoeShape = 'cone' | 'sphere' | 'line' | 'cube';

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
  id: string;
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
        action: boolean;
        bonus_action: boolean;
        reaction: boolean;
        movement_ft: number;
      };
      is_dead: boolean;
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
