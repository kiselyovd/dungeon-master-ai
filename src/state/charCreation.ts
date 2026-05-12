import type { StateCreator } from 'zustand';
import type { AbilityScores, InventoryItem } from './pc';

export type WizardTab =
  | 'class'
  | 'race'
  | 'background'
  | 'abilities'
  | 'skills'
  | 'spells'
  | 'equipment'
  | 'persona'
  | 'portrait'
  | 'review';

export type AbilityMethod = 'point_buy' | 'standard_array' | '4d6_drop_lowest';
export type EquipmentMode = 'package' | 'gold';

export interface EquipmentSlot {
  slotId: string;
  category: 'weapon' | 'armor' | 'gear' | 'pack' | 'ammo' | 'focus';
  itemId: string | null;
  customName: string | null;
  fromBackground: boolean;
}

export interface PersonalityFlag {
  source: 'background' | 'alignment' | 'race';
  flag: string;
}

export interface TestChatTurn {
  role: 'npc' | 'pc';
  text: string;
}

export interface CharacterDraft {
  classId: string | null;
  subclassId: string | null;
  raceId: string | null;
  subraceId: string | null;
  backgroundId: string | null;
  abilityMethod: AbilityMethod | null;
  abilities: AbilityScores;
  abilityRollHistory: number[][];
  pointBuyRemaining: number;
  skillProfs: string[];
  spells: { cantrips: string[]; level1: string[] };
  equipmentMode: EquipmentMode | null;
  equipmentSlots: EquipmentSlot[];
  equipmentInventory: InventoryItem[];
  goldRemaining: number;
  personalityFlags: PersonalityFlag[];
  ideals: string;
  bonds: string;
  flaws: string;
  backstory: string;
  name: string;
  alignment: string | null;
  portraitUrl: string | null;
  portraitPrompt: string | null;
}

export const EMPTY_DRAFT: CharacterDraft = {
  classId: null,
  subclassId: null,
  raceId: null,
  subraceId: null,
  backgroundId: null,
  abilityMethod: null,
  abilities: { str: 10, dex: 10, con: 10, int: 10, wis: 10, cha: 10 },
  abilityRollHistory: [],
  pointBuyRemaining: 27,
  skillProfs: [],
  spells: { cantrips: [], level1: [] },
  equipmentMode: null,
  equipmentSlots: [],
  equipmentInventory: [],
  goldRemaining: 0,
  personalityFlags: [],
  ideals: '',
  bonds: '',
  flaws: '',
  backstory: '',
  name: '',
  alignment: null,
  portraitUrl: null,
  portraitPrompt: null,
};

export interface CharCreationActions {
  setActiveTab: (tab: WizardTab) => void;
  setDraftField: <K extends keyof CharacterDraft>(key: K, value: CharacterDraft[K]) => void;
  setAbilityScore: (ability: keyof AbilityScores, value: number) => void;
  rollAbilityScores: () => void;
  applyAiSuggestion: (patch: Partial<CharacterDraft>) => void;
  setIsAssisting: (assisting: boolean) => void;
  resetDraft: () => void;
}

export interface CharCreationSlice {
  charCreation: CharacterDraft &
    CharCreationActions & {
      activeTab: WizardTab;
      isAssisting: boolean;
    };
}

const MAX_ROLL_ATTEMPTS = 3;

const DRAFT_KEYS = new Set(Object.keys(EMPTY_DRAFT) as Array<keyof CharacterDraft>);

function rollOneAbility(): number {
  const rolls: number[] = [1, 2, 3, 4].map(() => Math.floor(Math.random() * 6) + 1);
  rolls.sort((a, b) => a - b);
  // Drop the lowest (index 0), sum the remaining three.
  return (rolls[1] ?? 0) + (rolls[2] ?? 0) + (rolls[3] ?? 0);
}

function rollSix(): number[] {
  return [0, 1, 2, 3, 4, 5].map(() => rollOneAbility());
}

export const createCharCreationSlice: StateCreator<CharCreationSlice, [], [], CharCreationSlice> = (
  set,
) => ({
  charCreation: {
    ...EMPTY_DRAFT,
    activeTab: 'class',
    isAssisting: false,
    setActiveTab: (tab) => set((s) => ({ charCreation: { ...s.charCreation, activeTab: tab } })),
    setDraftField: (key, value) =>
      set((s) => ({ charCreation: { ...s.charCreation, [key]: value } })),
    setAbilityScore: (ability, value) =>
      set((s) => ({
        charCreation: {
          ...s.charCreation,
          abilities: { ...s.charCreation.abilities, [ability]: value },
        },
      })),
    rollAbilityScores: () =>
      set((s) => {
        if (s.charCreation.abilityRollHistory.length >= MAX_ROLL_ATTEMPTS) {
          return {};
        }
        return {
          charCreation: {
            ...s.charCreation,
            abilityRollHistory: [...s.charCreation.abilityRollHistory, rollSix()],
          },
        };
      }),
    applyAiSuggestion: (patch) =>
      set((s) => {
        const safe: Partial<CharacterDraft> = {};
        for (const [key, value] of Object.entries(patch)) {
          if (DRAFT_KEYS.has(key as keyof CharacterDraft)) {
            (safe as Record<string, unknown>)[key] = value;
          }
        }
        return { charCreation: { ...s.charCreation, ...safe } };
      }),
    setIsAssisting: (assisting) =>
      set((s) => ({ charCreation: { ...s.charCreation, isAssisting: assisting } })),
    resetDraft: () =>
      set((s) => ({
        charCreation: {
          ...s.charCreation,
          ...EMPTY_DRAFT,
          activeTab: 'class',
          isAssisting: false,
        },
      })),
  },
});
