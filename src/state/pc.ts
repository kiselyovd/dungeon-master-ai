import type { StateCreator } from 'zustand';

/**
 * Player character (PC) slice (M5 P2.14). Stores the canonical character
 * sheet shape: identity (race / class / background), level + XP, derived
 * combat stats (HP / AC / initiative / speed / proficiency bonus), the six
 * ability scores, saving-throw + skill proficiencies, and inventory.
 *
 * The slice does not own any combat-loop state - that belongs to the
 * combat slice and is mirrored back here only via small focused setters
 * (`setHp`, `setAc`). Higher-level "apply preset" overwrites the whole
 * snapshot for a given hero class - used when the onboarding wizard picks
 * a class so the rest of the app sees a fully-populated character sheet.
 *
 * `heroClass` is canonical D&D 5e class id ("fighter" / "wizard" /
 * "rogue" / "cleric"). `null` means "no hero created yet" - the rest of
 * the slice is also nullable so the CharacterSheet modal can render a
 * "Go to onboarding" empty state in that case.
 */

export type HeroClassId = 'fighter' | 'wizard' | 'rogue' | 'cleric';

export interface AbilityScores {
  str: number;
  dex: number;
  con: number;
  int: number;
  wis: number;
  cha: number;
}

export interface SkillProf {
  acrobatics?: boolean;
  athletics?: boolean;
  arcana?: boolean;
  deception?: boolean;
  history?: boolean;
  insight?: boolean;
  intimidation?: boolean;
  investigation?: boolean;
  perception?: boolean;
  persuasion?: boolean;
  stealth?: boolean;
  survival?: boolean;
}

export interface SavingThrowProf {
  str?: boolean;
  dex?: boolean;
  con?: boolean;
  int?: boolean;
  wis?: boolean;
  cha?: boolean;
}

export interface InventoryItem {
  /** Stable id - either uuid or a slug. Used for remove. */
  id: string;
  name: string;
  count: number;
  /** Matches a key in `Icons` (e.g. 'sword', 'bow', 'shield', 'potion', 'coin', 'scroll'). */
  icon?: string;
}

export interface PcData {
  heroClass: string | null;
  // Identity (null when no character has been created yet).
  name: string | null;
  race: string | null;
  /** D&D 5e archetype, e.g. 'Battle Master' for Fighter. */
  subclass: string | null;
  background: string | null;
  alignment: string | null;
  // Progression.
  level: number;
  experience: number;
  experienceNext: number;
  // Combat.
  hp: number;
  hpMax: number;
  ac: number;
  /** Signed initiative modifier. */
  initiative: number;
  speedFt: number;
  proficiencyBonus: number;
  // Sheet.
  abilities: AbilityScores;
  savingThrowProfs: SavingThrowProf;
  skillProfs: SkillProf;
  inventory: InventoryItem[];
}

export interface PcActions {
  setHeroClass: (heroClass: string | null) => void;
  /** Overwrite the whole PC with the preset for the given class. */
  applyPreset: (heroClass: HeroClassId) => void;
  setHp: (hp: number) => void;
  setAc: (ac: number) => void;
  addInventoryItem: (item: InventoryItem) => void;
  removeInventoryItem: (id: string) => void;
}

export interface PcSlice {
  pc: PcData & PcActions;
}

// ---- Helpers (pure, exported) -----------------------------------------

/** D&D 5e ability modifier from a raw score: floor((score - 10) / 2). */
export function abilityMod(score: number): number {
  return Math.floor((score - 10) / 2);
}

/**
 * Skill / saving-throw modifier helper. `prof` adds the proficiency bonus
 * on top of the underlying ability modifier; otherwise just the mod.
 */
export function skillMod(score: number, prof: boolean, profBonus: number): number {
  return abilityMod(score) + (prof ? profBonus : 0);
}

/** Same shape as `skillMod` - kept as a separate name for call-site clarity. */
export function savingThrowMod(score: number, prof: boolean, profBonus: number): number {
  return abilityMod(score) + (prof ? profBonus : 0);
}

// ---- Class presets -----------------------------------------------------

interface ClassPreset {
  name: string;
  race: string;
  subclass: string | null;
  background: string;
  alignment: string;
  level: number;
  experience: number;
  experienceNext: number;
  hp: number;
  hpMax: number;
  ac: number;
  initiative: number;
  speedFt: number;
  proficiencyBonus: number;
  abilities: AbilityScores;
  savingThrowProfs: SavingThrowProf;
  skillProfs: SkillProf;
  inventory: InventoryItem[];
}

function inv(id: string, name: string, count: number, icon: string): InventoryItem {
  return { id, name, count, icon };
}

const FIGHTER_PRESET: ClassPreset = {
  name: 'Hero',
  race: 'Human',
  subclass: null,
  background: 'Soldier',
  alignment: 'Lawful Neutral',
  level: 1,
  experience: 0,
  experienceNext: 300,
  hp: 12,
  hpMax: 12,
  ac: 16,
  initiative: 1,
  speedFt: 30,
  proficiencyBonus: 2,
  abilities: { str: 16, dex: 12, con: 14, int: 10, wis: 13, cha: 11 },
  savingThrowProfs: { str: true, con: true },
  skillProfs: { athletics: true, intimidation: true },
  inventory: [
    inv('fighter-longsword', 'Longsword', 1, 'sword'),
    inv('fighter-shield', 'Shield', 1, 'shield'),
    inv('fighter-potion-healing', 'Potion of Healing', 1, 'potion'),
    inv('fighter-gold', 'Gold pieces', 30, 'coin'),
  ],
};

const WIZARD_PRESET: ClassPreset = {
  name: 'Hero',
  race: 'High Elf',
  subclass: null,
  background: 'Sage',
  alignment: 'Neutral Good',
  level: 1,
  experience: 0,
  experienceNext: 300,
  hp: 8,
  hpMax: 8,
  ac: 12,
  initiative: 2,
  speedFt: 30,
  proficiencyBonus: 2,
  abilities: { str: 9, dex: 14, con: 12, int: 16, wis: 13, cha: 10 },
  savingThrowProfs: { int: true, wis: true },
  skillProfs: { arcana: true, history: true, investigation: true },
  inventory: [
    inv('wizard-quarterstaff', 'Quarterstaff', 1, 'sword'),
    inv('wizard-spellbook', 'Spellbook', 1, 'scroll'),
    inv('wizard-potion-healing', 'Potion of Healing', 1, 'potion'),
    inv('wizard-gold', 'Gold pieces', 20, 'coin'),
  ],
};

const ROGUE_PRESET: ClassPreset = {
  name: 'Hero',
  race: 'Halfling',
  subclass: null,
  background: 'Criminal',
  alignment: 'Chaotic Neutral',
  level: 1,
  experience: 0,
  experienceNext: 300,
  hp: 10,
  hpMax: 10,
  ac: 14,
  initiative: 3,
  speedFt: 30,
  proficiencyBonus: 2,
  abilities: { str: 10, dex: 16, con: 13, int: 12, wis: 11, cha: 14 },
  savingThrowProfs: { dex: true, int: true },
  skillProfs: {
    acrobatics: true,
    deception: true,
    perception: true,
    stealth: true,
  },
  inventory: [
    inv('rogue-shortbow', 'Shortbow', 1, 'bow'),
    inv('rogue-shortsword', 'Shortsword', 1, 'sword'),
    inv('rogue-potion-healing', 'Potion of Healing', 1, 'potion'),
    inv('rogue-gold', 'Gold pieces', 25, 'coin'),
  ],
};

const CLERIC_PRESET: ClassPreset = {
  name: 'Hero',
  race: 'Hill Dwarf',
  subclass: null,
  background: 'Acolyte',
  alignment: 'Lawful Good',
  level: 1,
  experience: 0,
  experienceNext: 300,
  hp: 11,
  hpMax: 11,
  ac: 16,
  initiative: 0,
  speedFt: 25,
  proficiencyBonus: 2,
  abilities: { str: 14, dex: 10, con: 14, int: 11, wis: 16, cha: 12 },
  savingThrowProfs: { wis: true, cha: true },
  skillProfs: { insight: true, persuasion: true },
  inventory: [
    inv('cleric-mace', 'Mace', 1, 'sword'),
    inv('cleric-shield', 'Shield', 1, 'shield'),
    inv('cleric-holy-symbol', 'Holy symbol', 1, 'scroll'),
    inv('cleric-potion-healing', 'Potion of Healing', 1, 'potion'),
    inv('cleric-gold', 'Gold pieces', 15, 'coin'),
  ],
};

const PRESETS: Record<HeroClassId, ClassPreset> = {
  fighter: FIGHTER_PRESET,
  wizard: WIZARD_PRESET,
  rogue: ROGUE_PRESET,
  cleric: CLERIC_PRESET,
};

function presetToData(klass: HeroClassId): Omit<PcData, 'heroClass'> & { heroClass: string } {
  const preset = PRESETS[klass];
  return {
    heroClass: klass,
    name: preset.name,
    race: preset.race,
    subclass: preset.subclass,
    background: preset.background,
    alignment: preset.alignment,
    level: preset.level,
    experience: preset.experience,
    experienceNext: preset.experienceNext,
    hp: preset.hp,
    hpMax: preset.hpMax,
    ac: preset.ac,
    initiative: preset.initiative,
    speedFt: preset.speedFt,
    proficiencyBonus: preset.proficiencyBonus,
    abilities: { ...preset.abilities },
    savingThrowProfs: { ...preset.savingThrowProfs },
    skillProfs: { ...preset.skillProfs },
    inventory: preset.inventory.map((it) => ({ ...it })),
  };
}

// ---- Empty / initial PC -----------------------------------------------

const EMPTY_ABILITIES: AbilityScores = {
  str: 10,
  dex: 10,
  con: 10,
  int: 10,
  wis: 10,
  cha: 10,
};

const EMPTY_PC: PcData = {
  heroClass: null,
  name: null,
  race: null,
  subclass: null,
  background: null,
  alignment: null,
  level: 1,
  experience: 0,
  experienceNext: 300,
  hp: 0,
  hpMax: 0,
  ac: 10,
  initiative: 0,
  speedFt: 30,
  proficiencyBonus: 2,
  abilities: { ...EMPTY_ABILITIES },
  savingThrowProfs: {},
  skillProfs: {},
  inventory: [],
};

function isHeroClassId(value: string | null): value is HeroClassId {
  return value === 'fighter' || value === 'wizard' || value === 'rogue' || value === 'cleric';
}

// ---- Slice -------------------------------------------------------------

export const createPcSlice: StateCreator<PcSlice, [], [], PcSlice> = (set) => ({
  pc: {
    ...EMPTY_PC,
    setHeroClass: (heroClass) =>
      set((s) => {
        // null -> wipe back to the empty PC (keeps the slice consistent).
        if (heroClass === null) {
          return {
            pc: {
              ...s.pc,
              ...EMPTY_PC,
            },
          };
        }
        // Known class id -> apply the preset so the rest of the sheet is filled.
        if (isHeroClassId(heroClass)) {
          return {
            pc: {
              ...s.pc,
              ...presetToData(heroClass),
            },
          };
        }
        // Unknown id - keep it as a string for forward-compat but do not seed.
        return {
          pc: { ...s.pc, heroClass },
        };
      }),
    applyPreset: (heroClass) =>
      set((s) => ({
        pc: {
          ...s.pc,
          ...presetToData(heroClass),
        },
      })),
    setHp: (hp) =>
      set((s) => {
        const max = s.pc.hpMax;
        const clamped = Math.max(0, Math.min(hp, max));
        return { pc: { ...s.pc, hp: clamped } };
      }),
    setAc: (ac) => set((s) => ({ pc: { ...s.pc, ac } })),
    addInventoryItem: (item) =>
      set((s) => ({
        pc: { ...s.pc, inventory: [...s.pc.inventory, item] },
      })),
    removeInventoryItem: (id) =>
      set((s) => ({
        pc: {
          ...s.pc,
          inventory: s.pc.inventory.filter((it) => it.id !== id),
        },
      })),
  },
});
