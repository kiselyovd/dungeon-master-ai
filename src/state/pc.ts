import type { StateCreator } from 'zustand';

/**
 * Minimal player character (PC) slice. v1 only persists the chosen class
 * captured during the onboarding wizard so the rest of the app can branch
 * on "is there a hero yet?" without dragging in the full character sheet
 * shape (Task 8 / P2.14 will own race, level, abilities, inventory, etc).
 *
 * `heroClass` is the canonical D&D 5e class id ("fighter" / "wizard" /
 * "rogue" / "cleric" today). Stored as `string | null` rather than a
 * picklist so the future P2.14 expansion can add classes without forcing a
 * persist-store migration step here. `null` means "no hero created yet".
 */

export type HeroClassId = 'fighter' | 'wizard' | 'rogue' | 'cleric';

export interface PcData {
  heroClass: string | null;
}

export interface PcActions {
  setHeroClass: (heroClass: string | null) => void;
}

export interface PcSlice {
  pc: PcData & PcActions;
}

export const createPcSlice: StateCreator<PcSlice, [], [], PcSlice> = (set) => ({
  pc: {
    heroClass: null,
    setHeroClass: (heroClass) =>
      set((s) => ({
        pc: { ...s.pc, heroClass },
      })),
  },
});
