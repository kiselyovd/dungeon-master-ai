import type { StateCreator } from 'zustand';
import type { AoeShape } from '../components/AoeTemplate';

export interface AoeTemplateEntry {
  id: string;
  shape: AoeShape;
  originX: number;
  originY: number;
  sizeInFt: number;
  school: string;
  rotateDeg: number;
  /** Unix ms timestamp; auto-removed when Date.now() >= expiresAt. */
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
}

export const DEFAULT_SPEED_FT = 30;

export interface CombatSlice {
  combat: {
    active: boolean;
    encounterId: string | null;
    tokens: CombatToken[];
    initiativeOrder: string[]; // ordered list of token ids
    currentTurnId: string | null;
    round: number;

    actionUsed: boolean;
    bonusUsed: boolean;
    reactionUsed: boolean;
    movementRemaining: number;

    aoeTemplates: AoeTemplateEntry[];

    startCombat: (encounterId: string, tokens: CombatToken[]) => void;
    endCombat: () => void;
    applyDamage: (tokenId: string, amount: number) => void;
    applyHealing: (tokenId: string, amount: number) => void;
    addCondition: (tokenId: string, condition: string) => void;
    removeCondition: (tokenId: string, condition: string) => void;
    setCurrentTurn: (tokenId: string | null) => void;
    advanceRound: () => void;
    moveToken: (tokenId: string, x: number, y: number) => void;
    addToken: (token: CombatToken) => void;
    updateToken: (tokenId: string, patch: Partial<CombatToken>) => void;
    removeToken: (tokenId: string) => void;

    useAction: () => void;
    useBonus: () => void;
    useReaction: () => void;
    moveBy: (distance: number) => void;
    endTurn: () => void;

    addAoeTemplate: (template: AoeTemplateEntry) => void;
    removeAoeTemplate: (id: string) => void;
  };
}

const econReset = () => ({
  actionUsed: false,
  bonusUsed: false,
  reactionUsed: false,
  movementRemaining: DEFAULT_SPEED_FT,
});

export const createCombatSlice: StateCreator<CombatSlice, [], [], CombatSlice> = (set) => ({
  combat: {
    active: false,
    encounterId: null,
    tokens: [],
    initiativeOrder: [],
    currentTurnId: null,
    round: 1,
    aoeTemplates: [],
    ...econReset(),

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
          ...econReset(),
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
          aoeTemplates: [],
          ...econReset(),
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
          ...econReset(),
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

    addToken: (token) =>
      set((s) => {
        if (s.combat.tokens.some((t) => t.id === token.id)) {
          return { combat: s.combat };
        }
        return {
          combat: {
            ...s.combat,
            tokens: [...s.combat.tokens, token],
            initiativeOrder: [...s.combat.initiativeOrder, token.id],
          },
        };
      }),

    updateToken: (tokenId, patch) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.map((t) => (t.id === tokenId ? { ...t, ...patch } : t)),
        },
      })),

    removeToken: (tokenId) =>
      set((s) => ({
        combat: {
          ...s.combat,
          tokens: s.combat.tokens.filter((t) => t.id !== tokenId),
          initiativeOrder: s.combat.initiativeOrder.filter((id) => id !== tokenId),
          currentTurnId: s.combat.currentTurnId === tokenId ? null : s.combat.currentTurnId,
        },
      })),

    useAction: () => set((s) => ({ combat: { ...s.combat, actionUsed: true } })),

    useBonus: () => set((s) => ({ combat: { ...s.combat, bonusUsed: true } })),

    useReaction: () => set((s) => ({ combat: { ...s.combat, reactionUsed: true } })),

    moveBy: (distance) =>
      set((s) => ({
        combat: {
          ...s.combat,
          movementRemaining: Math.max(0, s.combat.movementRemaining - distance),
        },
      })),

    endTurn: () =>
      set((s) => {
        if (s.combat.initiativeOrder.length === 0) {
          return { combat: s.combat };
        }
        const order = s.combat.initiativeOrder;
        const idx = s.combat.currentTurnId ? order.indexOf(s.combat.currentTurnId) : -1;
        const nextIdx = (idx + 1) % order.length;
        const nextId = order[nextIdx] ?? null;
        // A new round begins when the turn wraps past the last combatant back
        // to the top of the initiative order. idx < 0 (combat just started, no
        // current turn) is the first turn, not a wrap. [F1]
        const wrapped = idx >= 0 && nextIdx === 0;
        return {
          combat: {
            ...s.combat,
            currentTurnId: nextId,
            round: wrapped ? s.combat.round + 1 : s.combat.round,
            tokens: s.combat.tokens.map((t) => ({ ...t, isActive: t.id === nextId })),
            ...econReset(),
          },
        };
      }),

    addAoeTemplate: (template) =>
      set((s) => ({
        combat: { ...s.combat, aoeTemplates: [...s.combat.aoeTemplates, template] },
      })),

    removeAoeTemplate: (id) =>
      set((s) => ({
        combat: {
          ...s.combat,
          aoeTemplates: s.combat.aoeTemplates.filter((t) => t.id !== id),
        },
      })),
  },
});
