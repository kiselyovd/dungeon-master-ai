import type { CharacterDraft } from '../state/charCreation';
import { useStore } from '../state/useStore';

export function useCharacterDraft(): CharacterDraft {
  return useStore((s) => ({
    classId: s.charCreation.classId,
    subclassId: s.charCreation.subclassId,
    raceId: s.charCreation.raceId,
    subraceId: s.charCreation.subraceId,
    backgroundId: s.charCreation.backgroundId,
    abilityMethod: s.charCreation.abilityMethod,
    abilities: s.charCreation.abilities,
    abilityRollHistory: s.charCreation.abilityRollHistory,
    pointBuyRemaining: s.charCreation.pointBuyRemaining,
    skillProfs: s.charCreation.skillProfs,
    spells: s.charCreation.spells,
    equipmentMode: s.charCreation.equipmentMode,
    equipmentSlots: s.charCreation.equipmentSlots,
    equipmentInventory: s.charCreation.equipmentInventory,
    goldRemaining: s.charCreation.goldRemaining,
    personalityFlags: s.charCreation.personalityFlags,
    ideals: s.charCreation.ideals,
    bonds: s.charCreation.bonds,
    flaws: s.charCreation.flaws,
    backstory: s.charCreation.backstory,
    name: s.charCreation.name,
    alignment: s.charCreation.alignment,
    portraitUrl: s.charCreation.portraitUrl,
    portraitPrompt: s.charCreation.portraitPrompt,
  }));
}
