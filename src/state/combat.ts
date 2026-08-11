import type { StateCreator } from 'zustand';
import { projectionTokens } from '../features/combat/model/combatProjection';
import type {
  AoeTemplateEntry,
  CombatProjectionDto,
  CombatToken,
  SnapshotCombat,
} from '../features/combat/model/types';

export type {
  AoeShape,
  AoeTemplateEntry,
  CombatToken,
  SnapshotCombat,
} from '../features/combat/model/types';

export const DEFAULT_SPEED_FT = 30;

export interface CombatSlice {
  combat: {
    active: boolean;
    encounterId: string | null;
    revision: number;
    tokens: CombatToken[];
    initiativeOrder: string[];
    currentTurnId: string | null;
    round: number;
    actionUsed: boolean;
    bonusUsed: boolean;
    reactionUsed: boolean;
    movementRemaining: number;
    pendingCommands: Record<string, string>;
    aoeTemplates: AoeTemplateEntry[];
    replaceProjection: (projection: CombatProjectionDto) => boolean;
    markCommandPending: (commandId: string, kind: string) => void;
    settleCommand: (commandId: string) => void;
    clearProjection: () => void;
    hydrate: (snapshot: SnapshotCombat) => void;
    addAoeTemplate: (template: AoeTemplateEntry) => void;
    removeAoeTemplate: (id: string) => void;
  };
}

export function emptyCombatProjectionState() {
  return {
    active: false,
    encounterId: null,
    revision: -1,
    tokens: [] as CombatToken[],
    initiativeOrder: [] as string[],
    currentTurnId: null,
    round: 1,
    actionUsed: false,
    bonusUsed: false,
    reactionUsed: false,
    movementRemaining: 0,
    pendingCommands: {} as Record<string, string>,
  };
}

function derivedProjection(projection: CombatProjectionDto) {
  const tokens = projectionTokens(projection);
  const active = tokens.find((token) => token.id === projection.snapshot.current_combatant);
  return {
    active: projection.snapshot.active,
    encounterId: projection.encounter_id,
    revision: projection.revision,
    tokens,
    initiativeOrder: projection.snapshot.initiative.map((entry) => entry.id),
    currentTurnId: projection.snapshot.current_combatant,
    round: projection.snapshot.round,
    actionUsed: active ? active.actionAvailable === false : false,
    bonusUsed: active ? active.bonusAvailable === false : false,
    reactionUsed: active ? active.reactionAvailable === false : false,
    movementRemaining: active?.movementRemaining ?? 0,
  };
}

export function legacyCombatProjectionState(snapshot: SnapshotCombat) {
  const tokens: CombatToken[] = snapshot.tokens.map((token) => ({
    id: token.id,
    name: token.name,
    hp: token.hp,
    maxHp: token.max_hp,
    ac: token.ac,
    x: token.x,
    y: token.y,
    conditions: [...token.conditions],
    isActive: token.id === snapshot.current_turn_id,
    speed: DEFAULT_SPEED_FT,
    actionAvailable: true,
    bonusAvailable: true,
    reactionAvailable: true,
    movementRemaining: DEFAULT_SPEED_FT,
  }));
  return {
    active: snapshot.active,
    encounterId: snapshot.encounter_id,
    revision: 0,
    tokens,
    initiativeOrder: [...snapshot.initiative],
    currentTurnId: snapshot.current_turn_id,
    round: snapshot.round,
    actionUsed: false,
    bonusUsed: false,
    reactionUsed: false,
    movementRemaining: snapshot.active ? DEFAULT_SPEED_FT : 0,
    pendingCommands: {} as Record<string, string>,
  };
}

export const createCombatSlice: StateCreator<CombatSlice, [], [], CombatSlice> = (set, get) => ({
  combat: {
    ...emptyCombatProjectionState(),
    aoeTemplates: [],

    replaceProjection: (projection) => {
      const current = get().combat;
      if (
        current.encounterId === projection.encounter_id &&
        projection.revision <= current.revision
      ) {
        console.warn('[combat.projection]', {
          code: 'stale_combat_projection',
          encounterId: projection.encounter_id,
          currentRevision: current.revision,
          incomingRevision: projection.revision,
        });
        return false;
      }
      set((state) => ({
        combat: {
          ...state.combat,
          ...derivedProjection(projection),
        },
      }));
      return true;
    },

    markCommandPending: (commandId, kind) =>
      set((state) => ({
        combat: {
          ...state.combat,
          pendingCommands: { ...state.combat.pendingCommands, [commandId]: kind },
        },
      })),

    settleCommand: (commandId) =>
      set((state) => {
        const pendingCommands = { ...state.combat.pendingCommands };
        delete pendingCommands[commandId];
        return { combat: { ...state.combat, pendingCommands } };
      }),

    clearProjection: () =>
      set((state) => ({
        combat: { ...state.combat, ...emptyCombatProjectionState(), aoeTemplates: [] },
      })),

    hydrate: (snapshot) => {
      set((state) => ({
        combat: {
          ...state.combat,
          ...legacyCombatProjectionState(snapshot),
          aoeTemplates: [],
        },
      }));
    },

    addAoeTemplate: (template) =>
      set((state) => ({
        combat: { ...state.combat, aoeTemplates: [...state.combat.aoeTemplates, template] },
      })),
    removeAoeTemplate: (id) =>
      set((state) => ({
        combat: {
          ...state.combat,
          aoeTemplates: state.combat.aoeTemplates.filter((template) => template.id !== id),
        },
      })),
  },
});
