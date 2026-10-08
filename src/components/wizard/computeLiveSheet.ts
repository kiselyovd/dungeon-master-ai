import type { Compendium } from '../../api/srd';
import type { CharacterDraft } from '../../state/charCreation';
import type { AbilityScores } from '../../state/pc';
import { abilityMod, savingThrowMod, skillMod } from '../../state/pc';
import { type DefensePreview, deriveDefensePreview } from './defensePreview';
import { resolveDraftInventory } from './equipmentResolver';

export interface LiveSheetAbilityRow {
  score: number;
  mod: number;
}

export interface LiveSheetSavingThrow {
  mod: number;
  proficient: boolean;
}

export interface LiveSheetSkill {
  mod: number;
  proficient: boolean;
}

export interface LiveSheetInventoryItem {
  id: string;
  name: string;
  count: number;
  icon: string | null;
}

export interface LiveSheet {
  defense: DefensePreview | null;
  baseSpeedFt: number | null;
  name: string | null;
  className: string | null;
  raceName: string | null;
  backgroundName: string | null;
  subclassId: string | null;
  subraceName: string | null;
  level: number;
  hp: number | null;
  hpMax: number | null;
  ac: number | null;
  initiative: number | null;
  speedFt: number | null;
  proficiencyBonus: number;
  abilities: Record<keyof AbilityScores, LiveSheetAbilityRow>;
  savingThrows: Record<keyof AbilityScores, LiveSheetSavingThrow>;
  skills: Record<string, LiveSheetSkill>;
  inventoryPreview: LiveSheetInventoryItem[];
  inventoryOverflow: number;
  spellsPreview: { cantrips: string[]; level1: string[] } | null;
  placeholder: string | null;
}

const ABILITY_KEYS: (keyof AbilityScores)[] = ['str', 'dex', 'con', 'int', 'wis', 'cha'];

const SKILL_TO_ABILITY: Record<string, keyof AbilityScores> = {
  acrobatics: 'dex',
  animal_handling: 'wis',
  arcana: 'int',
  athletics: 'str',
  deception: 'cha',
  history: 'int',
  insight: 'wis',
  intimidation: 'cha',
  investigation: 'int',
  medicine: 'wis',
  nature: 'int',
  perception: 'wis',
  performance: 'cha',
  persuasion: 'cha',
  religion: 'int',
  sleight_of_hand: 'dex',
  stealth: 'dex',
  survival: 'wis',
};

const ALL_SKILLS = Object.keys(SKILL_TO_ABILITY);

/** Level 1 proficiency bonus is always 2 (PHB 5e). */
const LEVEL_1_PROFICIENCY_BONUS = 2;

interface MinimalCompendium {
  equipment?: Compendium['equipment'];
  races: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    speed: number;
    ability_score_increases: Partial<AbilityScores>;
    subraces: Array<{
      id: string;
      name_en: string;
      name_ru: string;
      additional_asi?: Partial<AbilityScores>;
    }>;
  }>;
  classes: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    hit_die: number;
    saving_throw_proficiencies: string[];
    armor_proficiencies?: string[];
  }>;
  backgrounds: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    skill_proficiencies: string[];
    ability_score_options: string[];
  }>;
}

function finalAbilityScore(
  base: number,
  ability: keyof AbilityScores,
  background: MinimalCompendium['backgrounds'][number] | null,
): number {
  // SRD 5.2.1 assigns ability increases through the background, not species.
  // Until the dedicated +2/+1 chooser lands, use the source order as a stable,
  // legal default: +2 to the first listed ability and +1 to the second.
  const options = background?.ability_score_options ?? [];
  if (options[0] === ability) return base + 2;
  if (options[1] === ability) return base + 1;
  return base;
}

export function computeLiveSheet(draft: CharacterDraft, compendium: MinimalCompendium): LiveSheet {
  const klass = draft.classId
    ? (compendium.classes.find((c) => c.id === draft.classId) ?? null)
    : null;
  const race = draft.raceId ? (compendium.races.find((r) => r.id === draft.raceId) ?? null) : null;
  const subrace =
    race && draft.subraceId
      ? (race.subraces.find((sr) => sr.id === draft.subraceId) ?? null)
      : null;
  const bg = draft.backgroundId
    ? (compendium.backgrounds.find((b) => b.id === draft.backgroundId) ?? null)
    : null;

  const abilities: Record<keyof AbilityScores, LiveSheetAbilityRow> = {} as never;
  for (const k of ABILITY_KEYS) {
    const finalScore = finalAbilityScore(draft.abilities[k], k, bg);
    abilities[k] = { score: finalScore, mod: abilityMod(finalScore) };
  }

  const proficiencyBonus = LEVEL_1_PROFICIENCY_BONUS;

  const savingThrows: Record<keyof AbilityScores, LiveSheetSavingThrow> = {} as never;
  const profSaves = new Set((klass?.saving_throw_proficiencies ?? []) as string[]);
  for (const k of ABILITY_KEYS) {
    const prof = profSaves.has(k);
    savingThrows[k] = {
      mod: savingThrowMod(abilities[k].score, prof, proficiencyBonus),
      proficient: prof,
    };
  }

  const skills: Record<string, LiveSheetSkill> = {};
  const bgSkills = new Set(bg?.skill_proficiencies ?? []);
  const draftSkills = new Set(draft.skillProfs);
  for (const skill of ALL_SKILLS) {
    const ability = SKILL_TO_ABILITY[skill] as keyof AbilityScores;
    const prof = bgSkills.has(skill) || draftSkills.has(skill);
    skills[skill] = {
      mod: skillMod(abilities[ability].score, prof, proficiencyBonus),
      proficient: prof,
    };
  }

  const conMod = abilities.con.mod;
  const hpMax = klass ? Math.max(1, klass.hit_die + conMod) : null;
  const hp = hpMax;

  const initiative = abilities.dex.mod;
  const baseSpeedFt = race ? race.speed : null;
  const items: LiveSheetInventoryItem[] = compendium.equipment
    ? resolveDraftInventory(draft, compendium as Compendium).map((item) => ({
        ...item,
        icon: item.icon ?? null,
      }))
    : [
        ...draft.equipmentSlots
          .filter((slot) => slot.itemId)
          .map((slot) => ({
            id: slot.slotId,
            name: slot.customName ?? slot.itemId ?? '',
            count: 1,
            icon: null,
          })),
        ...draft.equipmentInventory.map((item) => ({ ...item, icon: item.icon ?? null })),
      ];
  const defense =
    compendium.equipment && klass
      ? deriveDefensePreview(
          klass.id,
          klass.armor_proficiencies ?? [],
          Object.fromEntries(
            ABILITY_KEYS.map((key) => [key, abilities[key].score]),
          ) as unknown as AbilityScores,
          items.map(({ icon, ...item }) => ({ ...item, ...(icon === null ? {} : { icon }) })),
          compendium.equipment.armor,
          draft.defenseChoice,
        )
      : null;
  const ac = defense?.ac ?? 10 + abilities.dex.mod;
  const speedFt =
    baseSpeedFt === null ? null : Math.max(0, baseSpeedFt - (defense?.speedPenaltyFt ?? 0));

  const inventoryPreview = items.slice(0, 3);
  const inventoryOverflow = Math.max(0, items.length - 3);

  const hasSpells = draft.spells.cantrips.length > 0 || draft.spells.level1.length > 0;
  const spellsPreview = hasSpells
    ? { cantrips: [...draft.spells.cantrips], level1: [...draft.spells.level1] }
    : null;

  const placeholder = !klass ? 'pick_class_to_begin' : null;

  return {
    defense,
    baseSpeedFt,
    name: draft.name || null,
    className: klass ? klass.name_en : null,
    raceName: race ? race.name_en : null,
    subclassId: draft.subclassId,
    subraceName: subrace ? subrace.name_en : null,
    backgroundName: bg ? bg.name_en : null,
    level: 1,
    hp,
    hpMax,
    ac,
    initiative,
    speedFt,
    proficiencyBonus,
    abilities,
    savingThrows,
    skills,
    inventoryPreview,
    inventoryOverflow,
    spellsPreview,
    placeholder,
  };
}
