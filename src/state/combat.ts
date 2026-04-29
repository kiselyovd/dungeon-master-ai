import type { StateCreator } from 'zustand';

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
}

export interface CombatSlice {
  combat: {
    active: boolean;
    encounterId: string | null;
    tokens: CombatToken[];
    initiativeOrder: string[]; // ordered list of token ids
    currentTurnId: string | null;
    round: number;

    startCombat: (encounterId: string, tokens: CombatToken[]) => void;
    endCombat: () => void;
    applyDamage: (tokenId: string, amount: number) => void;
    applyHealing: (tokenId: string, amount: number) => void;
    addCondition: (tokenId: string, condition: string) => void;
    removeCondition: (tokenId: string, condition: string) => void;
    setCurrentTurn: (tokenId: string | null) => void;
    advanceRound: () => void;
    moveToken: (tokenId: string, x: number, y: number) => void;
  };
}

export const createCombatSlice: StateCreator<CombatSlice, [], [], CombatSlice> = (set) => ({
  combat: {
    active: false,
    encounterId: null,
    tokens: [],
    initiativeOrder: [],
    currentTurnId: null,
    round: 1,

    startCombat: (encounterId, tokens) =>
      set((s) => ({
        combat: {
          ...s.combat,
          active: true,
          encounterId,
          tokens,
          initiativeOrder: tokens.map((t) => t.id),
          currentTurnId: tokens[0]?.id ?? null,
          round: 1,
        },
      })),

    endCombat: () =>
      set((s) => ({
        combat: {
          ...s.combat,
          active: false,
          encounterId: null,
          tokens: [],
          initiativeOrder: [],
          currentTurnId: null,
          round: 1,
        },
      })),

    applyDamage: (tokenId, amount) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) =>
            t.id === tokenId ? { ...t, hp: Math.max(0, t.hp - amount) } : t,
          ),
        },
      })),

    applyHealing: (tokenId, amount) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) =>
            t.id === tokenId ? { ...t, hp: Math.min(t.maxHp, t.hp + amount) } : t,
          ),
        },
      })),

    addCondition: (tokenId, condition) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) =>
            t.id === tokenId && !t.conditions.includes(condition)
              ? { ...t, conditions: [...t.conditions, condition] }
              : t,
          ),
        },
      })),

    removeCondition: (tokenId, condition) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) =>
            t.id === tokenId
              ? { ...t, conditions: t.conditions.filter((c) => c !== condition) }
              : t,
          ),
        },
      })),

    setCurrentTurn: (tokenId) =>
      set((s) => ({
        combat: {
          ...s.combat,
          currentTurnId: tokenId,
          tokens: s.combat.tokens.map((t) => ({ ...t, isActive: t.id === tokenId })),
        },
      })),

    advanceRound: () => set((s) => ({ combat: { ...s.combat, round: s.combat.round + 1 } })),

    moveToken: (tokenId, x, y) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) => (t.id === tokenId ? { ...t, x, y } : t)),
        },
      })),
  },
});
