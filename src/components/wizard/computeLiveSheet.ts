import type { CharacterDraft } from '../../state/charCreation';
import type { AbilityScores } from '../../state/pc';

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
  name: string | null;
  className: string | null;
  raceName: string | null;
  backgroundName: string | null;
  subclassName: string | null;
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

function abilityMod(score: number): number {
  return Math.floor((score - 10) / 2);
}

interface MinimalCompendium {
  races: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    speed: number;
    ability_score_increases: Partial<AbilityScores>;
    subraces: Array<{ id: string; name_en: string; name_ru: string }>;
  }>;
  classes: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    hit_die: number;
    saving_throw_proficiencies: string[];
  }>;
  backgrounds: Array<{
    id: string;
    name_en: string;
    name_ru: string;
    skill_proficiencies: string[];
  }>;
}

export function computeLiveSheet(draft: CharacterDraft, compendium: MinimalCompendium): LiveSheet {
  const klass = draft.classId ? compendium.classes.find((c) => c.id === draft.classId) : null;
  const race = draft.raceId ? compendium.races.find((r) => r.id === draft.raceId) : null;
  const subrace =
    race && draft.subraceId
      ? (race.subraces.find((sr) => sr.id === draft.subraceId) ?? null)
      : null;
  const bg = draft.backgroundId
    ? compendium.backgrounds.find((b) => b.id === draft.backgroundId)
    : null;

  const abilities: Record<keyof AbilityScores, LiveSheetAbilityRow> = {} as never;
  for (const k of ABILITY_KEYS) {
    abilities[k] = { score: draft.abilities[k], mod: abilityMod(draft.abilities[k]) };
  }

  const proficiencyBonus = 2;

  const savingThrows: Record<keyof AbilityScores, LiveSheetSavingThrow> = {} as never;
  const profSaves = new Set((klass?.saving_throw_proficiencies ?? []) as string[]);
  for (const k of ABILITY_KEYS) {
    const prof = profSaves.has(k);
    savingThrows[k] = {
      mod: abilities[k].mod + (prof ? proficiencyBonus : 0),
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
      mod: abilities[ability].mod + (prof ? proficiencyBonus : 0),
      proficient: prof,
    };
  }

  const conMod = abilities.con.mod;
  const hpMax = klass ? klass.hit_die + conMod : null;
  const hp = hpMax;

  const initiative = klass ? abilities.dex.mod : null;
  const speedFt = race ? race.speed : null;

  const items: LiveSheetInventoryItem[] = [];
  for (const slot of draft.equipmentSlots) {
    if (slot.itemId) {
      items.push({
        id: slot.slotId,
        name: slot.customName ?? slot.itemId,
        count: 1,
        icon: null,
      });
    }
  }
  for (const it of draft.equipmentInventory) {
    items.push({ id: it.id, name: it.name, count: it.count, icon: it.icon ?? null });
  }
  const inventoryPreview = items.slice(0, 3);
  const inventoryOverflow = Math.max(0, items.length - 3);

  const hasSpells = draft.spells.cantrips.length > 0 || draft.spells.level1.length > 0;
  const spellsPreview = hasSpells ? draft.spells : null;

  const placeholder = !klass ? 'pick_class_to_begin' : null;

  return {
    name: draft.name || null,
    className: klass ? klass.name_en : null,
    raceName: race ? race.name_en : null,
    subclassName: draft.subclassId,
    subraceName: subrace ? subrace.name_en : null,
    backgroundName: bg ? bg.name_en : null,
    level: 1,
    hp,
    hpMax,
    ac: null,
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
