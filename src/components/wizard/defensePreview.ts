import type { DefenseChoice } from '../../api/contracts/defense';
import type { Armor } from '../../api/srd';
import type { AbilityScores, InventoryItem } from '../../state/pc';
import { abilityMod } from '../../state/pc';

export interface DefensePreview {
  choice: DefenseChoice;
  ac: number;
  speedPenaltyFt: number;
  stealthDisadvantage: boolean;
  valid: boolean;
  availableArmor: Armor[];
  shieldAvailable: boolean;
}

/** Mirrors the server's trained, owned defense subset; the server validates every commit. */
export function deriveDefensePreview(
  classId: string | null,
  training: string[],
  abilities: AbilityScores,
  inventory: InventoryItem[],
  armor: Armor[],
  explicit: DefenseChoice | null,
): DefensePreview {
  const owns = (id: string) =>
    inventory.some((item) => item.count > 0 && (item.id === id || item.id === `${classId}-${id}`));
  const availableArmor = armor.filter(
    (item) => item.category !== 'shield' && owns(item.id) && training.includes(item.category),
  );
  const shieldAvailable = owns('shield') && training.includes('shield');
  const dex = abilityMod(abilities.dex);
  const derive = (choice: DefenseChoice): DefensePreview => {
    const worn =
      choice.armor_id === null ? null : availableArmor.find((item) => item.id === choice.armor_id);
    const valid =
      choice.schema_version === 1 &&
      (choice.armor_id === null || !!worn) &&
      (!choice.shield || shieldAvailable);
    const unarmored =
      10 +
      dex +
      (classId === 'barbarian'
        ? abilityMod(abilities.con)
        : classId === 'monk' && !choice.shield
          ? abilityMod(abilities.wis)
          : 0);
    const dexBonus =
      worn?.ac_dex_bonus_cap === 0
        ? 0
        : worn?.ac_dex_bonus_cap == null
          ? dex
          : Math.min(dex, worn.ac_dex_bonus_cap);
    return {
      choice,
      valid,
      ac: (worn ? worn.ac_base + dexBonus : unarmored) + (choice.shield ? 2 : 0),
      speedPenaltyFt: worn?.str_req != null && abilities.str < worn.str_req ? 10 : 0,
      stealthDisadvantage: worn?.stealth_disadvantage ?? false,
      availableArmor,
      shieldAvailable,
    };
  };
  if (explicit) return derive(explicit);
  let best = derive({ schema_version: 1, armor_id: null, shield: shieldAvailable });
  for (const item of availableArmor) {
    const candidate = derive({ schema_version: 1, armor_id: item.id, shield: shieldAvailable });
    if (candidate.ac > best.ac) best = candidate;
  }
  return best;
}
