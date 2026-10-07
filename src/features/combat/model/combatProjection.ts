import * as v from 'valibot';
import { AdventureProjectionSchema } from '../../../api/contracts/adventure';
import { BattleMapSchema } from '../../../api/contracts/battleMap';
import type { CombatProjectionDto, CombatToken } from './types';

const isRecord = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === 'object' && !Array.isArray(value);
const isNumber = (value: unknown): value is number =>
  typeof value === 'number' && Number.isFinite(value);

export function parseCombatProjection(value: unknown): CombatProjectionDto | null {
  if (!isRecord(value) || !isRecord(value.snapshot)) return null;
  const snapshot = value.snapshot;
  if (value.authored !== undefined && value.authored !== null) {
    const authored = value.authored;
    if (
      !isRecord(authored) ||
      authored.kind !== 'bell_archive_defense' ||
      !v.safeParse(BattleMapSchema, authored.map).success ||
      !(
        authored.outcome === null ||
        ['active', 'held', 'retreated', 'defeated'].includes(String(authored.outcome))
      )
    )
      return null;
  }
  if (
    value.adventure !== undefined &&
    value.adventure !== null &&
    !v.safeParse(AdventureProjectionSchema, value.adventure).success
  )
    return null;
  if (value.campaign_scope !== undefined && value.campaign_scope !== null) {
    const scope = value.campaign_scope;
    if (
      !isRecord(scope) ||
      typeof scope.campaign_id !== 'string' ||
      typeof scope.session_id !== 'string' ||
      typeof scope.character_id !== 'string' ||
      !Number.isSafeInteger(scope.revision) ||
      (scope.revision as number) < 0
    )
      return null;
  }
  if (
    value.schema_version !== 1 ||
    typeof value.encounter_id !== 'string' ||
    !isNumber(value.revision) ||
    typeof snapshot.active !== 'boolean' ||
    !isNumber(snapshot.round) ||
    !(snapshot.current_combatant === null || typeof snapshot.current_combatant === 'string') ||
    !Array.isArray(snapshot.initiative) ||
    !Array.isArray(snapshot.combatants) ||
    !Array.isArray(value.events)
  )
    return null;

  for (const entry of snapshot.initiative) {
    if (
      !isRecord(entry) ||
      typeof entry.id !== 'string' ||
      !isNumber(entry.roll) ||
      !isNumber(entry.dex_tiebreak)
    )
      return null;
  }
  for (const combatant of snapshot.combatants) {
    if (
      !isRecord(combatant) ||
      !(
        combatant.display_name_key === undefined ||
        combatant.display_name_key === null ||
        typeof combatant.display_name_key === 'string'
      ) ||
      !(
        combatant.casting === undefined ||
        combatant.casting === null ||
        validCasting(combatant.casting)
      ) ||
      !(
        combatant.loadout === undefined ||
        combatant.loadout === null ||
        validLoadout(combatant.loadout)
      ) ||
      !(
        combatant.character_id === undefined ||
        combatant.character_id === null ||
        typeof combatant.character_id === 'string'
      ) ||
      typeof combatant.id !== 'string' ||
      typeof combatant.name !== 'string' ||
      !isNumber(combatant.max_hp) ||
      !isNumber(combatant.current_hp) ||
      !isNumber(combatant.temp_hp) ||
      !isNumber(combatant.ac) ||
      !isNumber(combatant.speed_ft) ||
      !isNumber(combatant.initiative_roll) ||
      !isNumber(combatant.dex_mod) ||
      !Array.isArray(combatant.conditions) ||
      !combatant.conditions.every((condition) => typeof condition === 'string') ||
      !isRecord(combatant.budget) ||
      typeof combatant.budget.action !== 'boolean' ||
      !(
        combatant.budget.spell_slot_used === undefined ||
        typeof combatant.budget.spell_slot_used === 'boolean'
      ) ||
      typeof combatant.budget.bonus_action !== 'boolean' ||
      typeof combatant.budget.reaction !== 'boolean' ||
      !isNumber(combatant.budget.movement_ft) ||
      typeof combatant.is_dead !== 'boolean' ||
      !isRecord(combatant.position) ||
      !isNumber(combatant.position.x) ||
      !isNumber(combatant.position.y)
    )
      return null;
  }
  return value as unknown as CombatProjectionDto;
}

export function projectionFromToolResult(result: unknown): CombatProjectionDto | null {
  if (!isRecord(result)) return null;
  return parseCombatProjection(result.projection);
}

export function projectionTokens(projection: CombatProjectionDto): CombatToken[] {
  const activeId = projection.snapshot.current_combatant;
  return projection.snapshot.combatants.map((combatant) => ({
    id: combatant.id,
    ...(combatant.display_name_key ? { displayNameKey: combatant.display_name_key } : {}),
    ...(combatant.character_id ? { characterId: combatant.character_id } : {}),
    ...(combatant.loadout ? { loadout: combatant.loadout } : {}),
    ...(combatant.casting ? { casting: combatant.casting } : {}),
    spellSlotUsed: combatant.budget.spell_slot_used ?? false,
    name: combatant.name,
    hp: combatant.current_hp,
    maxHp: combatant.max_hp,
    ac: combatant.ac,
    x: combatant.position.x,
    y: combatant.position.y,
    conditions: [...combatant.conditions],
    isActive: combatant.id === activeId,
    speed: combatant.speed_ft,
    actionAvailable: combatant.budget.action,
    bonusAvailable: combatant.budget.bonus_action,
    reactionAvailable: combatant.budget.reaction,
    movementRemaining: combatant.budget.movement_ft,
  }));
}

function validLoadout(value: unknown): boolean {
  if (
    !isRecord(value) ||
    typeof value.default_weapon !== 'string' ||
    !Array.isArray(value.attacks) ||
    !isRecord(value.quantities) ||
    !Number.isSafeInteger(value.arrows) ||
    (value.arrows as number) < 0 ||
    typeof value.shield !== 'boolean'
  )
    return false;
  if (
    !Object.values(value.quantities).every(
      (quantity) => Number.isSafeInteger(quantity) && (quantity as number) >= 0,
    )
  )
    return false;
  const dice = (value: unknown) =>
    isRecord(value) &&
    Number.isSafeInteger(value.count) &&
    (value.count as number) >= 0 &&
    typeof value.die === 'string' &&
    Number.isSafeInteger(value.modifier);
  return value.attacks.every(
    (attack) =>
      isRecord(attack) &&
      typeof attack.weapon_id === 'string' &&
      Number.isSafeInteger(attack.attack_bonus) &&
      dice(attack.damage) &&
      typeof attack.damage_type === 'string' &&
      ['reach_ft', 'normal_range_ft', 'long_range_ft'].every(
        (key) => Number.isSafeInteger(attack[key]) && (attack[key] as number) >= 0,
      ) &&
      ['ranged', 'thrown', 'ammunition', 'two_handed', 'heavy_disadvantage'].every(
        (key) => typeof attack[key] === 'boolean',
      ) &&
      (attack.versatile_damage === null || dice(attack.versatile_damage)),
  );
}

function validCasting(value: unknown): boolean {
  if (
    !isRecord(value) ||
    !['level', 'ability_modifier', 'attack_bonus', 'save_dc'].every((key) =>
      Number.isSafeInteger(value[key]),
    ) ||
    !Array.isArray(value.prepared) ||
    !value.prepared.every((id) => typeof id === 'string') ||
    !Array.isArray(value.choices) ||
    !isRecord(value.slots)
  )
    return false;
  return (
    value.choices.every(
      (choice) =>
        isRecord(choice) &&
        typeof choice.id === 'string' &&
        Number.isSafeInteger(choice.level) &&
        Number.isSafeInteger(choice.range_ft) &&
        ['bonus_action', 'healing', 'supported'].every((key) => typeof choice[key] === 'boolean'),
    ) &&
    Object.entries(value.slots).every(
      ([level, slots]) =>
        /^[1-9]$/.test(level) &&
        isRecord(slots) &&
        Number.isSafeInteger(slots.maximum) &&
        Number.isSafeInteger(slots.remaining) &&
        (slots.remaining as number) >= 0 &&
        (slots.remaining as number) <= (slots.maximum as number),
    )
  );
}
